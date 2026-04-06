import axios from 'axios'

export async function fetchStats() {
  const { data } = await axios.get('/api/stats')
  return data
}

export async function fetchTopDomains() {
  const { data } = await axios.get('/api/queries/top-domains?limit=10')
  return data
}

export async function fetchRecentQueries() {
  const { data } = await axios.get('/api/queries/recent?limit=50')
  return data
}
