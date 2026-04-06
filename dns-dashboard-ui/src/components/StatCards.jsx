export default function StatCards({ stats }) {
  const { total, blocked, avg_latency_ms } = stats
  const blockRate = total > 0 ? ((blocked / total) * 100).toFixed(1) : '0.0'
  const latency = avg_latency_ms != null ? avg_latency_ms.toFixed(1) : '—'

  return (
    <>
      <style>{`
        .stat-cards {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 16px;
        }
        .stat-card {
          background: #1a1d27;
          border-radius: 8px;
          padding: 20px 24px;
        }
        .stat-card__label {
          font-size: 12px;
          color: #94a3b8;
          text-transform: uppercase;
          letter-spacing: 0.05em;
          margin-bottom: 8px;
        }
        .stat-card__value {
          font-size: 32px;
          font-weight: 700;
          color: #4f9eff;
        }
        .stat-card__sub {
          font-size: 12px;
          color: #64748b;
          margin-top: 4px;
        }
      `}</style>
      <div className="stat-cards">
        <div className="stat-card">
          <div className="stat-card__label">Total Queries</div>
          <div className="stat-card__value">{total}</div>
        </div>
        <div className="stat-card">
          <div className="stat-card__label">Blocked Queries</div>
          <div className="stat-card__value">{blocked}</div>
          <div className="stat-card__sub">{blockRate}% block rate</div>
        </div>
        <div className="stat-card">
          <div className="stat-card__label">Avg Latency</div>
          <div className="stat-card__value">{latency} ms</div>
        </div>
      </div>
    </>
  )
}
