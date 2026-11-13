import { cn } from '../lib/cn'

/*
  The OpenStream TV lockup, drawn from the outlined brand artwork in
  design/logo. The file carries the ink wordmark, the ring and the on-air dot;
  scaling is height-only so the proportions set by the brand stay intact.
*/
export function Logo({ className, size = 17 }: { className?: string; size?: number }) {
  return (
    <img
      src="/brand/openstream-lockup.svg"
      alt="OpenStream TV"
      width={Math.round(size * 6.63)}
      height={size}
      className={cn('block w-auto shrink-0', className)}
      style={{ height: size }}
    />
  )
}
