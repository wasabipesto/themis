// Formats an input amount as currency ($X,XXX,XXX).
export function formatVolume(amount: number): string {
  return "$" + (amount / 1).toFixed(0).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

// Rounds to nearest tenth and prefixes a sign if it's not already present.
export function formatRelScore(amount: number): string {
  const rounded = Math.round(amount * 10) / 10;
  return (rounded >= 0 ? "+" : "") + rounded.toFixed(1);
}

// Rounds to nearest hundredth and returns a string.
export function formatAbsScore(amount: number): string {
  const rounded = Math.round(amount * 100) / 100;
  return rounded.toFixed(2);
}
