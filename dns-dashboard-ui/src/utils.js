/**
 * Groups an array of query objects by the minute of their UTC timestamp.
 * @param {Array<{timestamp: string}>} queries
 * @returns {Array<{time: string, count: number}>} sorted chronologically
 */
export function bucketByMinute(queries) {
  const counts = {}
  for (const q of queries) {
    const d = new Date(q.timestamp)
    const hh = String(d.getUTCHours()).padStart(2, '0')
    const mm = String(d.getUTCMinutes()).padStart(2, '0')
    const key = `${hh}:${mm}`
    counts[key] = (counts[key] ?? 0) + 1
  }
  return Object.entries(counts)
    .sort(([a], [b]) => (a < b ? -1 : 1))
    .map(([time, count]) => ({ time, count }))
}
