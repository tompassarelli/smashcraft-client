// The steps the Play panel lists after a press that was refused or failed: the lines the run collected, then the reason Play gives.
export function stepsAfterError(lines: string[], error: unknown): string[] {
  const reason = String(error);
  return lines.includes(reason) ? lines : [...lines, reason];
}
