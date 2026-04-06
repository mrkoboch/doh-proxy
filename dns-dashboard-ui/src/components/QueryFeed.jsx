function formatTime(isoString) {
  const d = new Date(isoString)
  const hh = String(d.getUTCHours()).padStart(2, '0')
  const mm = String(d.getUTCMinutes()).padStart(2, '0')
  const ss = String(d.getUTCSeconds()).padStart(2, '0')
  return `${hh}:${mm}:${ss}`
}

export default function QueryFeed({ queries }) {
  return (
    <>
      <style>{`
        .query-feed {
          background: #1a1d27;
          border-radius: 8px;
          overflow: hidden;
          display: flex;
          flex-direction: column;
          max-height: 480px;
        }
        .query-feed__title {
          padding: 16px 20px;
          font-size: 14px;
          font-weight: 600;
          color: #94a3b8;
          text-transform: uppercase;
          letter-spacing: 0.05em;
          border-bottom: 1px solid #2d3148;
        }
        .query-feed__scroll {
          overflow-y: auto;
          flex: 1;
        }
        .query-feed table {
          width: 100%;
          border-collapse: collapse;
          font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
          font-size: 12px;
        }
        .query-feed th {
          padding: 8px 12px;
          text-align: left;
          color: #64748b;
          font-weight: 500;
          background: #0f1117;
          position: sticky;
          top: 0;
        }
        .query-feed td {
          padding: 7px 12px;
          color: #e2e8f0;
          border-bottom: 1px solid #1e2235;
          white-space: nowrap;
          max-width: 200px;
          overflow: hidden;
          text-overflow: ellipsis;
        }
        .query-feed tr:hover td {
          background: #1e2235;
        }
        .badge {
          padding: 2px 6px;
          border-radius: 4px;
          font-size: 10px;
          font-weight: 700;
          letter-spacing: 0.05em;
        }
        .badge--clean { background: #14532d; color: #4ade80; }
        .badge--blocked { background: #4c0519; color: #f87171; }
        .empty-state {
          padding: 32px;
          text-align: center;
          color: #475569;
          font-size: 14px;
        }
      `}</style>
      <div className="query-feed">
        <div className="query-feed__title">Recent Queries</div>
        {queries.length === 0 ? (
          <div className="empty-state">No queries yet.</div>
        ) : (
          <div className="query-feed__scroll">
            <table>
              <thead>
                <tr>
                  <th>Time</th>
                  <th>Domain</th>
                  <th>Type</th>
                  <th>Latency</th>
                  <th>Blocked</th>
                  <th>Resolver</th>
                </tr>
              </thead>
              <tbody>
                {queries.map((q) => (
                  <tr key={q.id}>
                    <td>{formatTime(q.timestamp)}</td>
                    <td title={q.domain}>{q.domain}</td>
                    <td>{q.query_type}</td>
                    <td>{q.latency_ms != null ? `${q.latency_ms}ms` : '—'}</td>
                    <td>
                      {q.blocked ? (
                        <span className="badge badge--blocked">BLOCKED</span>
                      ) : (
                        <span className="badge badge--clean">CLEAN</span>
                      )}
                    </td>
                    <td title={q.resolver ?? ''}>{q.resolver ?? '—'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </>
  )
}
