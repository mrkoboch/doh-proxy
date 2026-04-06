import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from 'recharts'

export default function TopDomains({ domains }) {
  return (
    <>
      <style>{`
        .top-domains {
          background: #1a1d27;
          border-radius: 8px;
          padding: 20px;
        }
        .top-domains__title {
          font-size: 14px;
          font-weight: 600;
          color: #94a3b8;
          text-transform: uppercase;
          letter-spacing: 0.05em;
          margin-bottom: 16px;
        }
        .top-domains__empty {
          color: #475569;
          font-size: 14px;
          text-align: center;
          padding: 32px 0;
        }
      `}</style>
      <div className="top-domains">
        <div className="top-domains__title">Top Domains</div>
        {domains.length === 0 ? (
          <div className="top-domains__empty">No data yet.</div>
        ) : (
          <ResponsiveContainer width="100%" height={domains.length * 36 + 20}>
            <BarChart
              data={domains}
              layout="vertical"
              margin={{ top: 0, right: 24, bottom: 0, left: 8 }}
            >
              <XAxis
                type="number"
                tick={{ fill: '#64748b', fontSize: 11 }}
                axisLine={false}
                tickLine={false}
              />
              <YAxis
                type="category"
                dataKey="domain"
                width={160}
                tick={{ fill: '#e2e8f0', fontSize: 12, fontFamily: 'monospace' }}
                axisLine={false}
                tickLine={false}
              />
              <Tooltip
                contentStyle={{
                  background: '#1a1d27',
                  border: '1px solid #2d3148',
                  borderRadius: 6,
                  color: '#e2e8f0',
                }}
                cursor={{ fill: 'rgba(79,158,255,0.08)' }}
              />
              <Bar dataKey="count" radius={[0, 4, 4, 0]}>
                {domains.map((entry) => (
                  <Cell key={entry.domain} fill="#4f9eff" />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        )}
      </div>
    </>
  )
}
