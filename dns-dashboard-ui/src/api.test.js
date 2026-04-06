import { describe, it, expect, vi, beforeEach } from 'vitest'
import axios from 'axios'
import { fetchStats, fetchTopDomains, fetchRecentQueries } from './api'

vi.mock('axios')

beforeEach(() => {
  vi.resetAllMocks()
})

describe('fetchStats', () => {
  it('calls GET /api/stats and returns data', async () => {
    const data = { total: 10, blocked: 2, avg_latency_ms: 15.5 }
    axios.get.mockResolvedValue({ data })
    const result = await fetchStats()
    expect(axios.get).toHaveBeenCalledWith('/api/stats')
    expect(result).toEqual(data)
  })
})

describe('fetchTopDomains', () => {
  it('calls GET /api/queries/top-domains?limit=10 and returns data', async () => {
    const data = [{ domain: 'example.com', count: 5 }]
    axios.get.mockResolvedValue({ data })
    const result = await fetchTopDomains()
    expect(axios.get).toHaveBeenCalledWith('/api/queries/top-domains?limit=10')
    expect(result).toEqual(data)
  })
})

describe('fetchRecentQueries', () => {
  it('calls GET /api/queries/recent?limit=50 and returns data', async () => {
    const data = [{ id: 1, domain: 'a.com', query_type: 'A' }]
    axios.get.mockResolvedValue({ data })
    const result = await fetchRecentQueries()
    expect(axios.get).toHaveBeenCalledWith('/api/queries/recent?limit=50')
    expect(result).toEqual(data)
  })
})
