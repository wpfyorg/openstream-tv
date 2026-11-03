param(
    [ValidateSet('full', 'slim')][string]$Variant,
    [string]$Version = 'latest',
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Programs\OpenStream TV'),
    [string]$Repo = 'wpfyorg/openstream-tv',
    [switch]$NoTls
)
$ErrorActionPreference = 'Stop'

function Get-OpenStreamEnv([string]$Primary, [string]$Legacy, [string]$Default = '') {
    $value = [Environment]::GetEnvironmentVariable($Primary, 'Process')
    if (-not [string]::IsNullOrEmpty($value)) { return $value }
    $value = [Environment]::GetEnvironmentVariable($Legacy, 'Process')
    if (-not [string]::IsNullOrEmpty($value)) { return $value }
    return $Default
}

if ((Get-OpenStreamEnv 'OPENSTREAM_INSTALL_TLS' 'JIOTV_INSTALL_TLS' '1') -eq '0') { $NoTls = $true }
if (-not $PSBoundParameters.ContainsKey('Version')) { $Version = Get-OpenStreamEnv 'OPENSTREAM_VERSION' 'JIOTV_VERSION' $Version }
if (-not $PSBoundParameters.ContainsKey('Repo')) { $Repo = Get-OpenStreamEnv 'OPENSTREAM_REPO' 'JIOTV_REPO' $Repo }
if (-not $PSBoundParameters.ContainsKey('InstallDir')) {
    $configuredInstallDir = Get-OpenStreamEnv 'OPENSTREAM_INSTALL_DIR' 'JIOTV_INSTALL_DIR'
    if ($configuredInstallDir) { $InstallDir = $configuredInstallDir }
}

$VariantExplicit = $PSBoundParameters.ContainsKey('Variant')
$envVariant = Get-OpenStreamEnv 'OPENSTREAM_VARIANT' 'JIOTV_VARIANT'
if ($envVariant) {
    if ($envVariant -notin @('full', 'slim')) { throw 'OPENSTREAM_VARIANT must be full or slim' }
    if (-not $VariantExplicit) { $Variant = $envVariant }
    $VariantExplicit = $true
}
if (-not $Variant) { $Variant = 'full' }
$Onboard = Get-OpenStreamEnv 'OPENSTREAM_ONBOARD' 'JIOTV_ONBOARD' 'auto'
if ($Onboard -notin @('auto', 'always', 'never')) { throw 'OPENSTREAM_ONBOARD must be auto, always or never' }
$Provider = Get-OpenStreamEnv 'OPENSTREAM_PROVIDER' 'JIOTV_PROVIDER'
if ($Provider -in @('tv+', 'tvplus')) { $Provider = 'extras' }
if ($Provider -notin @('', 'jiotv', 'extras', 'fastway')) { throw 'OPENSTREAM_PROVIDER must be jiotv, tv+ or fastway' }
$PublicHost = Get-OpenStreamEnv 'OPENSTREAM_PUBLIC_HOST' 'JIOTV_PUBLIC_HOST'
$Tunnel = Get-OpenStreamEnv 'OPENSTREAM_TUNNEL' 'JIOTV_TUNNEL'
if ($Tunnel -notin @('', 'none', 'temp', 'named')) { throw 'OPENSTREAM_TUNNEL must be none, temp, or named' }
$TunnelToken = Get-OpenStreamEnv 'OPENSTREAM_TUNNEL_TOKEN' 'JIOTV_TUNNEL_TOKEN'

# Same rule as the shell installer: only a real console drives the flow, so
# redirected runs (CI, scheduled tasks) keep the old non-interactive output.
# The output is deliberately ASCII: Windows PowerShell 5.1 reads a BOM-less
# script as ANSI, which would mangle box-drawing characters in the source.
$Interactive = $Onboard -ne 'never' -and -not [Console]::IsInputRedirected -and -not [Console]::IsOutputRedirected

#region onboarding-ui
function Write-Rule { Write-Host ('  ' + ('-' * 44)) -ForegroundColor DarkGray }
function Write-Ok([string]$Text) { Write-Host "  + $Text" -ForegroundColor Green }
function Write-Warn2([string]$Text) { Write-Host "  ! $Text" -ForegroundColor Yellow }
function Write-Fail([string]$Text) { Write-Host "  x $Text" -ForegroundColor Red }
function Write-Step([string]$Text) { Write-Host ''; Write-Host "  * $Text" -ForegroundColor Cyan; Write-Host '' }

function Set-UserEnvironment([string]$Name, [AllowNull()][string]$Value) {
    if ([string]::IsNullOrEmpty($Value)) {
        [Environment]::SetEnvironmentVariable($Name, $null, 'User')
        Remove-Item -LiteralPath "Env:$Name" -ErrorAction SilentlyContinue
        return
    }
    [Environment]::SetEnvironmentVariable($Name, $Value, 'User')
    Set-Item -LiteralPath "Env:$Name" -Value $Value
}

function Show-Banner([string]$Subtitle) {
    Write-Host ''
    Write-Host '  OpenStream TV ' -ForegroundColor Cyan -NoNewline
    Write-Host 'installer ' -NoNewline
    Write-Host " $Subtitle" -ForegroundColor DarkGray
    Write-Rule
}

# Arrow keys, j/k, digits and Enter. Enter is the only commit key, so a stray
# keypress never installs something the user did not choose.
function Select-Option {
    param([string]$Question, [object[]]$Options, [int]$Default = 0)
    if ($Default -lt 0 -or $Default -ge $Options.Count) { $Default = 0 }
    Write-Host "  $Question" -ForegroundColor Cyan
    Write-Host ''
    $index = $Default
    $top = [Console]::CursorTop
    $width = 60
    $draw = {
        [Console]::SetCursorPosition(0, $top)
        for ($i = 0; $i -lt $Options.Count; $i++) {
            $pad = ' ' * [Math]::Max(0, $width - $Options[$i].Label.Length)
            if ($i -eq $index) {
                Write-Host '  > ' -ForegroundColor Cyan -NoNewline
                Write-Host $Options[$i].Label -ForegroundColor White -NoNewline
                Write-Host $pad
            }
            else {
                Write-Host ('    ' + $Options[$i].Label + $pad) -ForegroundColor DarkGray
            }
        }
    }
    & $draw
    while ($true) {
        $key = [Console]::ReadKey($true)
        switch ($key.Key) {
            'UpArrow' { if ($index -gt 0) { $index-- } }
            'DownArrow' { if ($index -lt $Options.Count - 1) { $index++ } }
            'K' { if ($index -gt 0) { $index-- } }
            'J' { if ($index -lt $Options.Count - 1) { $index++ } }
            'Enter' { & $draw; Write-Ok $Options[$index].Label; return $Options[$index].Value }
            default {
                $digit = 0
                if ([int]::TryParse([string]$key.KeyChar, [ref]$digit) -and $digit -ge 1 -and $digit -le $Options.Count) {
                    $index = $digit - 1
                }
            }
        }
        & $draw
    }
}

function Read-Text([string]$Question, [string]$Default = '') {
    $prompt = if ($Default) { "  $Question [$Default]: " } else { "  ${Question}: " }
    Write-Host $prompt -ForegroundColor Cyan -NoNewline
    $answer = [Console]::ReadLine()
    if ([string]::IsNullOrWhiteSpace($answer)) { $answer = $Default }
    return $answer.Trim()
}

function Read-Secret([string]$Question) {
    Write-Host "  ${Question}: " -ForegroundColor Cyan -NoNewline
    $secure = Read-Host -AsSecureString
    return [Runtime.InteropServices.Marshal]::PtrToStringAuto(
        [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure))
}

# The provider logins read plain stdin lines, so they are driven through pipes
# instead of being handed the console: the wizard keeps its own prompts and the
# one-time password is typed once, here. stdout and stderr are polled so the
# wizard knows the OTP has actually been sent rather than guessing with a delay.
function New-Dialog {
    param([string]$Exe, [string]$Arguments)
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $Exe
    $psi.Arguments = $Arguments
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $utf8 = New-Object System.Text.UTF8Encoding $false
    $psi.StandardOutputEncoding = $utf8
    $psi.StandardErrorEncoding = $utf8
    if ($psi.PSObject.Properties['StandardInputEncoding']) { $psi.StandardInputEncoding = $utf8 }
    $process = [System.Diagnostics.Process]::Start($psi)
    return [pscustomobject]@{
        Process = $process
        Text    = New-Object System.Text.StringBuilder
        Streams = @{
            Out = @{ Reader = $process.StandardOutput; Buf = (New-Object byte[] 4096); Task = $null; Done = $false }
            Err = @{ Reader = $process.StandardError; Buf = (New-Object byte[] 4096); Task = $null; Done = $false }
        }
    }
}

# Pulls whatever output has arrived so far into the dialog's text buffer.
function Read-DialogChunk {
    param($Dialog)
    foreach ($name in @('Out', 'Err')) {
        $state = $Dialog.Streams[$name]
        if ($state.Done) { continue }
        if ($null -eq $state.Task) {
            $state.Task = $state.Reader.BaseStream.ReadAsync($state.Buf, 0, $state.Buf.Length)
        }
        if ($state.Task.Wait(0)) {
            $count = $state.Task.Result
            $state.Task = $null
            if ($count -gt 0) {
                [void]$Dialog.Text.Append([Text.Encoding]::UTF8.GetString($state.Buf, 0, $count))
            }
            else {
                $state.Done = $true
            }
        }
    }
}

function Wait-Dialog {
    param($Dialog, [string]$Pattern, [int]$TimeoutSec = 45)
    $clock = [Diagnostics.Stopwatch]::StartNew()
    while ($clock.Elapsed.TotalSeconds -lt $TimeoutSec) {
        Read-DialogChunk $Dialog
        if ($Dialog.Text.ToString() -match $Pattern) { return $true }
        if ($Dialog.Process.HasExited -and $Dialog.Streams['Out'].Done -and $Dialog.Streams['Err'].Done) { return $false }
        Start-Sleep -Milliseconds 100
    }
    return $false
}

function Send-Dialog {
    param($Dialog, [string]$Line)
    # The command can exit before it reads the line; that is a failed sign-in,
    # reported by the caller, not a reason to abort the wizard.
    try {
        $Dialog.Process.StandardInput.WriteLine($Line)
        $Dialog.Process.StandardInput.Flush()
    }
    catch { }
}

function Complete-Dialog {
    param($Dialog, [int]$TimeoutSec = 60)
    try { $Dialog.Process.StandardInput.Close() } catch { }
    if (-not $Dialog.Process.HasExited) {
        if (-not $Dialog.Process.WaitForExit($TimeoutSec * 1000)) {
            try { $Dialog.Process.Kill() } catch { }
            return $false
        }
    }
    $clock = [Diagnostics.Stopwatch]::StartNew()
    while ($clock.Elapsed.TotalSeconds -lt 5) {
        Read-DialogChunk $Dialog
        if ($Dialog.Streams['Out'].Done -and $Dialog.Streams['Err'].Done) { break }
        Start-Sleep -Milliseconds 50
    }
    return $Dialog.Process.ExitCode -eq 0
}

function Show-Dialog {
    param($Dialog)
    $text = $Dialog.Text.ToString().TrimEnd()
    if (-not $text) { return }
    Write-Host ''
    foreach ($line in ($text -split "`r?`n")) { Write-Host "  | $line" -ForegroundColor DarkGray }
    Write-Host ''
}

function Test-MobileNumber([string]$Number) { return $Number -match '^[0-9]{6,}$' }

function Connect-JioTv {
    param([string]$Exe)
    Write-Step 'Sign in to JioTV'
    $number = Read-Text 'Mobile number registered with JioTV (+91)'
    if (-not (Test-MobileNumber $number)) { Write-Fail 'A mobile number is required for the OTP'; return $false }
    $dialog = New-Dialog $Exe 'login otp'
    Send-Dialog $dialog $number
    if (Wait-Dialog $dialog 'OTP sent') {
        Write-Ok "OTP sent to +91$number"
        Send-Dialog $dialog (Read-Secret 'Enter the OTP')
    }
    if ((Complete-Dialog $dialog) -and ($dialog.Text.ToString() -match 'Login successful')) {
        Write-Ok 'Signed in to JioTV'
        return $true
    }
    Show-Dialog $dialog
    Write-Fail 'JioTV sign-in did not complete'
    return $false
}

function Connect-Extras {
    param([string]$Exe)
    Write-Step 'Sign in to JioTV+'
    Write-Host '  Use the JioFiber or AirFiber login for this connection.' -ForegroundColor DarkGray
    $number = Read-Text 'Mobile number registered to the fibre account (+91)'
    if (-not (Test-MobileNumber $number)) { Write-Fail 'A mobile number is required for the OTP'; return $false }
    $cmd = if ($script:TvPlusCmd) { $script:TvPlusCmd } else { 'extras login' }
    $dialog = New-Dialog $Exe $cmd
    Send-Dialog $dialog $number
    # A number with no selectable connections goes straight to the OTP prompt.
    if (Wait-Dialog $dialog 'Choose a connection:|OTP:') {
        if ($dialog.Text.ToString() -match 'Choose a connection:') {
            $found = [regex]::Matches($dialog.Text.ToString(), '(?m)^\s{2}([0-9]+)\.\s+(.+?)\s*$')
            if ($found.Count -gt 0) {
                $choices = @()
                foreach ($match in $found) { $choices += @{ Value = $match.Groups[1].Value; Label = $match.Groups[2].Value } }
                Send-Dialog $dialog ([string](Select-Option 'Choose the connection' $choices))
            }
            $null = Wait-Dialog $dialog 'OTP:'
        }
        Write-Ok "OTP sent to +91$number"
        Send-Dialog $dialog (Read-Secret 'Enter the OTP')
    }
    if ((Complete-Dialog $dialog) -and ($dialog.Text.ToString() -match 'login saved')) {
        Write-Ok 'Signed in to JioTV+'
        return $true
    }
    Show-Dialog $dialog
    Write-Fail 'JioTV+ sign-in did not complete'
    return $false
}

function Connect-Fastway {
    param([string]$Exe)
    Write-Step 'Sign in to Fastway'
    $user = Read-Text 'Fastway username'
    if (-not $user) { Write-Fail 'A username is required'; return $false }
    $password = Read-Secret 'Fastway password'
    if (-not $password) { Write-Fail 'A password is required'; return $false }
    $dialog = New-Dialog $Exe 'fastway login'
    Send-Dialog $dialog $user
    Send-Dialog $dialog $password
    if ((Complete-Dialog $dialog) -and ($dialog.Text.ToString() -match 'login saved')) {
        Write-Ok 'Signed in to Fastway'
        return $true
    }
    Show-Dialog $dialog
    Write-Fail 'Fastway sign-in did not complete'
    return $false
}

# Sign-in is part of the flow, so a failure is retried before the install is
# reported as incomplete.
function Connect-Provider {
    param([string]$Exe, [string]$Provider, [int]$Attempts = 3)
    for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
        $ok = $false
        try {
            if ($Provider -eq 'extras') { $ok = Connect-Extras $Exe }
            elseif ($Provider -eq 'fastway') { $ok = Connect-Fastway $Exe }
            else { $ok = Connect-JioTv $Exe }
        }
        catch {
            Write-Fail "Sign-in could not run: $($_.Exception.Message)"
            $ok = $false
        }
        if ($ok) { return $true }
        if ($attempt -lt $Attempts) {
            $retry = Select-Option 'Try again?' @(
                @{ Value = $true; Label = 'Yes' },
                @{ Value = $false; Label = 'No' }
            )
            if (-not $retry) { return $false }
        }
    }
    return $false
}
#endregion onboarding-ui

function Get-LanAddress {
    try {
        $route = Get-NetRoute -DestinationPrefix '0.0.0.0/0' -ErrorAction Stop |
            Sort-Object RouteMetric | Select-Object -First 1
        $address = Get-NetIPAddress -InterfaceIndex $route.InterfaceIndex -AddressFamily IPv4 -ErrorAction Stop |
            Select-Object -First 1
        if ($address) { return $address.IPAddress }
    }
    catch { }
    return 'localhost'
}

$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
$target = switch ($arch) {
    'X64' { 'x86_64-pc-windows-msvc' }
    'X86' { 'i686-pc-windows-msvc' }
    'Arm64' { 'aarch64-pc-windows-msvc' }
    default { throw "Unsupported Windows architecture: $arch" }
}

if ($Interactive -and -not $VariantExplicit) {
    Show-Banner $arch
    $defaultVariant = @('full', 'slim').IndexOf($Variant)
    $Variant = Select-Option 'Which build?' @(
        @{ Value = 'full'; Label = 'Full   browser UI plus IPTV' },
        @{ Value = 'slim'; Label = 'Slim   IPTV only, no browser UI' }
    ) $defaultVariant
}

$preferredAsset = "openstream-$Variant-$target.exe"
$legacyAsset = "jiotv-$Variant-$target.exe"
if ($Version -eq 'latest') { $base = "https://github.com/$Repo/releases/latest/download" }
else {
    if (-not $Version.StartsWith('v')) { $Version = "v$Version" }
    $base = "https://github.com/$Repo/releases/download/$Version"
}
$temp = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temp | Out-Null
try {
    $sums = Join-Path $temp 'SHA256SUMS'
    Invoke-WebRequest -Uri "$base/SHA256SUMS" -OutFile $sums
    $asset = $preferredAsset
    $entry = Get-Content $sums | Where-Object { $_ -match "^\s*([0-9a-fA-F]{64})\s+\*?$([regex]::Escape($asset))\s*$" } | Select-Object -First 1
    if (-not $entry) {
        $asset = $legacyAsset
        $entry = Get-Content $sums | Where-Object { $_ -match "^\s*([0-9a-fA-F]{64})\s+\*?$([regex]::Escape($asset))\s*$" } | Select-Object -First 1
        if ($entry) { Write-Warn2 "Using legacy release asset $asset" }
    }
    if (-not $entry) { throw "SHA256SUMS has no entry for $preferredAsset or $legacyAsset" }
    $binary = Join-Path $temp $asset
    Invoke-WebRequest -Uri "$base/$asset" -OutFile $binary
    $expected = [regex]::Match($entry, '^\s*([0-9a-fA-F]{64})').Groups[1].Value.ToLowerInvariant()
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $binary).Hash.ToLowerInvariant()
    if ($actual -ne $expected) { throw "Checksum mismatch for $asset" }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    $destination = Join-Path $InstallDir 'openstream.exe'
    $alreadyInstalled = Test-Path -LiteralPath $destination
    $keepExistingAccount = $false
    if ($Interactive -and $alreadyInstalled) {
        Write-Step 'OpenStream TV is already installed here'
        $update = Select-Option 'Update it in place and keep the current account?' @(
            @{ Value = $true; Label = 'Yes' },
            @{ Value = $false; Label = 'No' }
        )
        if (-not $update) { Write-Ok 'Left as it was'; return }
        $keepExistingAccount = $true
    }
    Copy-Item -LiteralPath $binary -Destination $destination -Force
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = @($userPath -split ';' | Where-Object { $_ })
    if (-not ($entries | Where-Object { $_.TrimEnd('\') -ieq $InstallDir.TrimEnd('\') })) {
        [Environment]::SetEnvironmentVariable('Path', (($entries + $InstallDir) -join ';'), 'User')
        $env:Path = "$InstallDir;$env:Path"
    }
    Write-Output "Installed openstream ($Variant, $target) to $destination"

    $serveLine = 'openstream serve --host 0.0.0.0'
    if (-not $NoTls) { $serveLine += ' --tls' }

    if ($Interactive) {
        if ($keepExistingAccount) {
            Write-Ok 'Updated the binary; existing account and environment settings were kept'
            Write-Host "  Start it with: $serveLine"
            return
        }

        $openstream = $destination
        $help = ''
        try { $help = (& $openstream --help 2>&1 | Out-String) } catch { $help = '' }
        # `tv+ login` is the current name; older binaries still answer to
        # `extras login`, so that is used when it is the only one advertised.
        $TvPlusCmd = ''
        if ($help -match 'tv\+ login') { $TvPlusCmd = 'tv+ login' }
        elseif ($help -match 'extras login') { $TvPlusCmd = 'extras login' }
        $providers = @(@{ Value = 'jiotv'; Label = 'JioTV       live channels and programme guide - any network' })
        if ($TvPlusCmd) { $providers += @{ Value = 'extras'; Label = 'JioTV+      extra channels, movies and shows - JioFiber/AirFiber' } }
        if ($help -match 'fastway login') { $providers += @{ Value = 'fastway'; Label = 'Fastway     live TV from a Fastway subscription - Fastway/ACT' } }

        if (-not ($providers.Value -contains $Provider)) { $Provider = '' }
        if (-not $Provider) {
            if ($providers.Count -eq 1) {
                $Provider = $providers[0].Value
                $only = if ($Provider -eq 'extras') { 'JioTV+' } else { $Provider }
                Write-Step "Using $only (the only provider this build supports)"
            }
            else {
                Write-Host '  Which login you have depends on your network: JioTV works anywhere,' -ForegroundColor DarkGray
                Write-Host '  JioTV+ needs a JioFiber or AirFiber login, Fastway needs Fastway or ACT.' -ForegroundColor DarkGray
                $Provider = Select-Option 'Which account are you setting up?' $providers
            }
        }

        Write-Step 'Address other devices will use'
        if (-not $PublicHost) { $PublicHost = Read-Text "This machine's address on your network" (Get-LanAddress) }

        if (-not $Tunnel) {
            Write-Step 'Access from outside your network'
            $Tunnel = Select-Option 'Open OpenStream TV beyond your LAN?' @(
                @{ Value = 'none'; Label = 'No, LAN only' },
                @{ Value = 'temp'; Label = 'Temporary Cloudflare URL, new address every start' },
                @{ Value = 'named'; Label = 'Permanent Cloudflare tunnel, needs a tunnel token' }
            )
        }
        if ($Tunnel -eq 'named') {
            Write-Host '  Create a tunnel at dash.cloudflare.com, point its public hostname at' -ForegroundColor DarkGray
            Write-Host '  http://localhost:5001, then copy the token from the tunnel' -ForegroundColor DarkGray
            Write-Warn2 'Anyone with that hostname can reach the sign-in page. Keep the admin password strong.'
            if (-not $TunnelToken) { $TunnelToken = Read-Secret 'Cloudflare tunnel token' }
            if (-not $TunnelToken.StartsWith('eyJ')) {
                Write-Warn2 'That does not look like a tunnel token; using a temporary URL instead'
                $Tunnel = 'temp'
                $TunnelToken = ''
            }
        }
        elseif ($Tunnel -eq 'temp') {
            Write-Warn2 'Anyone with the address can reach the sign-in page. Keep the admin password strong.'
        }
        if ($Tunnel -ne 'none') { $serveLine += ' --tunnel' }

        $tvPlusEnabled = if ($Provider -eq 'extras') { 'true' } else { 'false' }
        $fastwayEnabled = if ($Provider -eq 'fastway') { 'true' } else { 'false' }
        Set-UserEnvironment 'OPENSTREAM_TVPLUS' $tvPlusEnabled
        Set-UserEnvironment 'OPENSTREAM_FASTWAY' $fastwayEnabled
        if ($Tunnel -eq 'named' -and $TunnelToken) {
            Set-UserEnvironment 'OPENSTREAM_TUNNEL_TOKEN' $TunnelToken
        }
        else {
            Set-UserEnvironment 'OPENSTREAM_TUNNEL_TOKEN' $null
        }
        $providerEnv = switch ($Provider) {
            'extras'  { "`$env:OPENSTREAM_TVPLUS = 'true'; `$env:OPENSTREAM_FASTWAY = 'false'; " }
            'fastway' { "`$env:OPENSTREAM_TVPLUS = 'false'; `$env:OPENSTREAM_FASTWAY = 'true'; " }
            default   { "`$env:OPENSTREAM_TVPLUS = 'false'; `$env:OPENSTREAM_FASTWAY = 'false'; " }
        }
        $serveLine = "$providerEnv$serveLine"

        # Sign-in happens inside the wizard: the commands read plain stdin
        # lines, so the number and the OTP are typed here, once.
        $signedIn = Connect-Provider $openstream $Provider
        $loginCmd = switch ($Provider) {
            'extras' { $TvPlusCmd }
            'fastway' { 'fastway login' }
            default { 'login otp' }
        }
        if ($signedIn) { Write-Ok 'Signed in' }
        else { Write-Fail "Sign-in did not complete; run 'openstream $loginCmd' when you are ready" }

        if ($Variant -eq 'full') {
            Write-Step 'Protect the browser interface'
            $password = Read-Secret 'Choose an admin password'
            if ($password) {
                $password | & $openstream admin password | Out-Null
                if ($LASTEXITCODE -eq 0) { Write-Ok 'Admin password saved' }
                else { Write-Fail 'Could not save the password; run: openstream admin password' }
            }
            else { Write-Warn2 'No password set; run: openstream admin password' }
        }

        Write-Host ''
        Write-Rule
        Write-Host '  Installation complete' -ForegroundColor Green
        Write-Rule
        Write-Host "  build     $Variant $target" -ForegroundColor DarkGray
        Write-Host "  account   $(if ($Provider -eq 'extras') { 'JioTV+' } else { $Provider })"
        if (-not $signedIn) { Write-Warn2 "  Not signed in yet: run 'openstream $loginCmd'" }
        if ($Variant -eq 'full') {
            if ($NoTls) { Write-Host "  browser   http://${PublicHost}:5001/" }
            else { Write-Host "  browser   https://${PublicHost}:5443/" }
        }
        $playlistPath = (& $openstream key show 2>$null | Out-String).Trim()
        if ($playlistPath.StartsWith('http://') -or $playlistPath.StartsWith('https://')) {
            $playlistUrl = $playlistPath
        }
        elseif ($playlistPath.StartsWith('/')) {
            $playlistUrl = "http://${PublicHost}:5001$playlistPath"
        }
        elseif ($playlistPath) {
            $playlistUrl = "http://${PublicHost}:5001/$playlistPath"
        }
        else {
            $playlistUrl = ''
        }
        if ($playlistUrl) { Write-Host "  playlist  $playlistUrl" }
        else { Write-Warn2 "Could not get the authenticated playlist URL; run 'openstream key show' after starting the server" }
        if ($Tunnel -ne 'none') { Write-Host '  remote    a Cloudflare tunnel address' }
        Write-Host ''
        if ($TunnelToken) {
            Write-Host '  The tunnel token was saved to your user environment without printing it.'
        }
        Write-Host "  Start it with: $serveLine"
        Write-Host '  Run this installer again to change the account or the build.'
        Write-Host ''
        return
    }

    if ($NoTls) {
        Write-Output 'Next: openstream login otp; openstream admin password; openstream serve --host 0.0.0.0'
        if ($Variant -eq 'slim') { Write-Output 'IPTV apps: use the playlist URL that openstream serve prints when it starts, with localhost:5001 on this PC or <this-pc-ip>:5001 from other devices (see ipconfig). The slim build has no browser UI.' }
        else {
            Write-Output 'Browser UI on this PC: http://localhost:5001/ (browsers need HTTPS or localhost for DRM and encrypted HLS playback; add --tls to enable HTTPS)'
            Write-Output 'From other devices on your network: http://<this-pc-ip>:5001/ (see ipconfig)'
        }
    }
    else {
        Write-Output 'Next: openstream login otp; openstream admin password; openstream serve --host 0.0.0.0 --tls'
        if ($Variant -eq 'slim') { Write-Output 'IPTV apps: use the playlist URL that openstream serve prints when it starts, with localhost:5001 on this PC or <this-pc-ip>:5001 from other devices (see ipconfig). The slim build has no browser UI.' }
        else {
            Write-Output 'Browser UI on this PC (HTTPS, self-signed certificate; accept the one-time warning): https://localhost:5443/'
            Write-Output 'From other devices on your network: https://<this-pc-ip>:5443/ (browser), http://<this-pc-ip>:5001/ (IPTV apps, plain HTTP playlist); see ipconfig for the address'
        }
    }
}
finally { Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue }
