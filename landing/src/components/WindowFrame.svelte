<script>
  let { title, class: className = '', children } = $props()
</script>

<!-- A desktop window: traffic lights and a title bar around any content. -->
<div class="overflow-hidden rounded-[10px] border-[2.5px] border-[#2a2f3d] bg-surface shadow-window {className}">
  <div class="flex h-[46px] items-center gap-[7px] border-b border-line bg-[#fbfbfc] px-5">
    <span aria-hidden="true" class="size-[13px] rounded-full bg-light-red"></span>
    <span aria-hidden="true" class="size-[13px] rounded-full bg-light-yellow"></span>
    <span aria-hidden="true" class="size-[13px] rounded-full bg-light-green"></span>
    <span class="ml-[18px] text-[15px] text-muted">{title}</span>
  </div>
  {@render children()}
</div>
