import { describe, it, expect } from 'vitest'
import { bucketByMinute } from './utils'

describe('bucketByMinute', () => {
  it('returns empty array for empty input', () => {
    expect(bucketByMinute([])).toEqual([])
  })

  it('groups queries into 1-minute buckets', () => {
    const queries = [
      { timestamp: '2026-04-06T12:00:05Z' },
      { timestamp: '2026-04-06T12:00:45Z' },
      { timestamp: '2026-04-06T12:01:10Z' },
    ]
    const result = bucketByMinute(queries)
    expect(result).toHaveLength(2)
    expect(result[0].count).toBe(2) // 12:00
    expect(result[1].count).toBe(1) // 12:01
  })

  it('labels each bucket as HH:MM', () => {
    const queries = [{ timestamp: '2026-04-06T09:07:00Z' }]
    const result = bucketByMinute(queries)
    expect(result[0].time).toBe('09:07')
  })

  it('sorts buckets chronologically oldest-first', () => {
    const queries = [
      { timestamp: '2026-04-06T12:02:00Z' },
      { timestamp: '2026-04-06T12:00:00Z' },
      { timestamp: '2026-04-06T12:01:00Z' },
    ]
    const result = bucketByMinute(queries)
    expect(result.map(r => r.time)).toEqual(['12:00', '12:01', '12:02'])
  })
})
