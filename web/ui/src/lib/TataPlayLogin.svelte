<script>
  import { tataplaySendOtp, tataplayVerifyOtp } from "./api.js";

  let { ondone } = $props();
  let step = $state("number");
  let number = $state("");
  let accounts = $state([]);
  let refId = $state("");
  let otp = $state("");
  let message = $state("");
  let busy = $state(false);

  async function run(fn) {
    busy = true;
    message = "";
    try {
      await fn();
    } catch (err) {
      message = err.message;
    } finally {
      busy = false;
    }
  }

  // Tata Play offers one login reference per account on the number, so the
  // user picks one when there is more than a single choice.
  function accountLabel(account) {
    return [account.nativeName, account.accountServiceType]
      .filter(Boolean)
      .join(" · ");
  }

  const sendOTP = (e) => {
    e.preventDefault();
    run(async () => {
      const d = await tataplaySendOtp(number);
      if (d?.status) {
        accounts = d.accounts ?? [];
        refId = accounts[0]?.refId ?? "";
        if (accounts.length) step = "otp";
        else message = "No Tata Play account was offered for this number.";
      } else {
        message = "Couldn't send the Tata Play OTP. Check the number.";
      }
    });
  };

  const verify = (e) => {
    e.preventDefault();
    run(async () => {
      const d = await tataplayVerifyOtp({ otp, refId });
      if (d?.status) ondone();
      else message = "The OTP is wrong or has expired.";
    });
  };
</script>

{#if step === "number"}
  <form onsubmit={sendOTP}>
    <input class="input" type="tel" inputmode="numeric" placeholder="10-digit mobile number" pattern={'[0-9]{10}'} maxlength="10" bind:value={number} required />
    <button class="btn primary" disabled={busy}>Send OTP</button>
  </form>
{:else}
  <form onsubmit={verify}>
    {#if accounts.length > 1}
      <select class="input" bind:value={refId} aria-label="Tata Play account">
        {#each accounts as account}
          <option value={account.refId}>{accountLabel(account)}</option>
        {/each}
      </select>
    {/if}
    <input class="input" inputmode="numeric" autocomplete="one-time-code" placeholder="OTP" bind:value={otp} required />
    <div class="row">
      <button class="btn primary" disabled={busy}>Log in</button>
      <button class="btn" type="button" onclick={() => (step = "number")}>Back</button>
    </div>
  </form>
{/if}
<p class="error" role="alert">{message}</p>

<style>
  form { display: flex; flex-direction: column; gap: 10px; width: 100%; align-items: flex-start; }
  .row { display: flex; gap: 8px; }
  p { margin: 0; }
</style>
