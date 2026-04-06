import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import QueryFeed from './QueryFeed'

const sampleQueries = [
  {
    id: 1,
    timestamp: '2026-04-06T09:07:42Z',
    domain: 'example.com',
    query_type: 'A',
    latency_ms: 35,
    blocked: false,
    resolver: 'https://dns.example/dns-query',
  },
  {
    id: 2,
    timestamp: '2026-04-06T09:07:55Z',
    domain: 'ads.tracker.io',
    query_type: 'AAAA',
    latency_ms: null,
    blocked: true,
    resolver: null,
  },
]

describe('QueryFeed', () => {
  it('renders domain names', () => {
    render(<QueryFeed queries={sampleQueries} />)
    expect(screen.getByText('example.com')).toBeInTheDocument()
    expect(screen.getByText('ads.tracker.io')).toBeInTheDocument()
  })

  it('formats timestamp as HH:MM:SS', () => {
    render(<QueryFeed queries={sampleQueries} />)
    expect(screen.getByText('09:07:42')).toBeInTheDocument()
    expect(screen.getByText('09:07:55')).toBeInTheDocument()
  })

  it('renders latency with ms suffix', () => {
    render(<QueryFeed queries={sampleQueries} />)
    expect(screen.getByText('35ms')).toBeInTheDocument()
  })

  it('renders — for null latency', () => {
    render(<QueryFeed queries={sampleQueries} />)
    expect(screen.getAllByText('—').length).toBeGreaterThan(0)
  })

  it('renders BLOCKED badge for blocked queries', () => {
    render(<QueryFeed queries={sampleQueries} />)
    expect(screen.getByText('BLOCKED')).toBeInTheDocument()
  })

  it('renders CLEAN badge for non-blocked queries', () => {
    render(<QueryFeed queries={sampleQueries} />)
    expect(screen.getByText('CLEAN')).toBeInTheDocument()
  })

  it('renders resolver URL when present', () => {
    render(<QueryFeed queries={sampleQueries} />)
    expect(screen.getByText('https://dns.example/dns-query')).toBeInTheDocument()
  })

  it('renders empty state message when queries is empty', () => {
    render(<QueryFeed queries={[]} />)
    expect(screen.getByText('No queries yet.')).toBeInTheDocument()
  })
})
