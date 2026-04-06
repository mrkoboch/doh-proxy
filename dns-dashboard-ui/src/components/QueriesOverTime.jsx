import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from 'recharts'
import { bucketByMinute } from '../utils'

export default function QueriesOverTime({ queries }) {
  const data = bucketByMinute(queries)

  return (
    <>
      <style>{`
        .queries-over-time {
          background: #1a1d27;
          border-radius: 8px;
          padding: 20px;
        }
        .queries-over-time__title {
          font-size: 14px;
          font-weight: 600;
          color: #94a3b8;
          text-transform: uppercase;
          letter-spacing: 0.05em;
          margin-bottom: 16px;
        }
        .queries-over-time__empty {
          color: #475569;
          font-size: 14px;
          text-align: center;
          padding: 32px 0;
        }
      `}</style>
      <div className="queries-over-time">
        <div className="queries-over-time__title">Queries Over Time</div>
        {data.length === 0 ? (
          <div className="queries-over-time__empty">No data yet.</div>
        ) : (
          <ResponsiveContainer width="100%" height={200}>
            <LineChart data={data} margin={{ top: 4, right: 24, bottom: 0, left: 0 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="#1e2235" />
              <XAxis
                dataKey="time"
                tick={{ fill: '#64748b', fontSize: 11 }}
                axisLine={false}
                tickLine={false}
              />
              <YAxis
                allowDecimals={false}
                tick={{ fill: '#64748b', fontSize: 11 }}
                axisLine={false}
                tickLine={false}
                width={28}
              />
              <Tooltip
                contentStyle={{
                  background: '#1a1d27',
                  border: '1px solid #2d3148',
                  borderRadius: 6,
                  color: '#e2e8f0',
                }}
              />
              <Line
                type="monotone"
                dataKey="count"
                stroke="#4f9eff"
                strokeWidth={2}
                dot={false}
                activeDot={{ r: 4, fill: '#4f9eff' }}
              />
            </LineChart>
          </ResponsiveContainer>
        )}
      </div>
    </>
  )
}
