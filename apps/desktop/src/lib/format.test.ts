import { describe, expect, it } from 'vitest';
import { formatBytes, timeAgo } from './format.ts';

describe('timeAgo', () => {
  const now = new Date('2026-07-27T12:00:00Z').getTime();

  it('collapses anything very recent to "now"', () => {
    expect(timeAgo(now - 1_000, now)).toBe('now');
    expect(timeAgo(now - 44_000, now)).toBe('now');
  });

  it('steps up through minutes, hours and days', () => {
    expect(timeAgo(now - 5 * 60_000, now)).toBe('5m');
    expect(timeAgo(now - 3 * 3_600_000, now)).toBe('3h');
    expect(timeAgo(now - 2 * 86_400_000, now)).toBe('2d');
  });

  it('falls back to a date past a week', () => {
    // Beyond 7 days the relative form stops being useful.
    expect(timeAgo(now - 30 * 86_400_000, now)).toMatch(/\d/);
    expect(timeAgo(now - 30 * 86_400_000, now)).not.toMatch(/^\d+[mhd]$/);
  });

  it('never renders a negative age for a clock that ran backwards', () => {
    // Clock skew or an NTP correction can put createdAt slightly in the future.
    expect(timeAgo(now + 60_000, now)).toBe('now');
  });
});

describe('formatBytes', () => {
  it('scales through the units', () => {
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(2048)).toBe('2.0 KB');
    expect(formatBytes(5 * 1024 * 1024)).toBe('5.0 MB');
  });
});
