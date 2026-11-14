<script>
  let { size = 17, class: className = '' } = $props()
</script>

<!--
  The OpenStream TV lockup, drawn from the outlined brand artwork in
  design/logo. The file carries the ink wordmark, the ring and the on-air dot;
  scaling is height-only so the proportions set by the brand stay intact.
-->
<img
  src="/brand/openstream-lockup.svg"
  alt="OpenStream TV"
  width={Math.round(size * 6.63)}
  height={size}
  class="block w-auto shrink-0 {className}"
  style="height: {size}px"
/>
