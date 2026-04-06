import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import StatCards from './StatCards'

const sampleStats = { total: 100, blocked: 25, avg_latency_ms: 42.5 }

describe('StatCards', () => {
  it('renders total queries count', () => {
    render(<StatCards stats={sampleStats} />)
    expect(screen.getByText('100')).toBeInTheDocument()
  })

  it('renders blocked queries count', () => {
    render(<StatCards stats={sampleStats} />)
    expect(screen.getByText('25')).toBeInTheDocument()
  })

  it('renders block rate percentage', () => {
    render(<StatCards stats={sampleStats} />)
    expect(screen.getByText('25.0% block rate')).toBeInTheDocument()
  })

  it('renders avg latency rounded to 1 decimal', () => {
    render(<StatCards stats={sampleStats} />)
    expect(screen.getByText('42.5 ms')).toBeInTheDocument()
  })

  it('renders — when avg_latency_ms is null', () => {
    render(<StatCards stats={{ total: 0, blocked: 0, avg_latency_ms: null }} />)
    expect(screen.getByText('— ms')).toBeInTheDocument()
  })

  it('renders 0.0% block rate when total is 0', () => {
    render(<StatCards stats={{ total: 0, blocked: 0, avg_latency_ms: null }} />)
    expect(screen.getByText('0.0% block rate')).toBeInTheDocument()
  })
})
