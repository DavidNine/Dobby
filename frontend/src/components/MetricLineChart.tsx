// F5 — MetricLineChart
//
// Thin wrapper over react-chartjs-2 <Line>. Pure presentational: maps a
// HistoryPoint[] into Chart.js data (x = formatTimeAxis(timestamp, range),
// y = the chosen numeric field) and renders it. No data fetching.

import { Line } from 'react-chartjs-2'
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Tooltip,
  Legend,
} from 'chart.js'
import type { HistoryPoint, Range } from '../api/types'
import { formatTimeAxis } from '../utils/format'

// Register the Chart.js building blocks a Line chart needs, once at module load.
ChartJS.register(CategoryScale, LinearScale, PointElement, LineElement, Tooltip, Legend)

export type MetricField = 'cpu_percent' | 'mem_percent' | 'net_rx_bps' | 'net_tx_bps'

export interface MetricLineChartProps {
  title: string
  points: HistoryPoint[]
  field: MetricField
  range: Range
}

export default function MetricLineChart({ title, points, field, range }: MetricLineChartProps) {
  const labels = points.map((p) => formatTimeAxis(p.timestamp, range))
  const values = points.map((p) => p[field])

  const data = {
    labels,
    datasets: [
      {
        label: title,
        data: values,
        borderColor: 'rgb(59, 130, 246)',
        backgroundColor: 'rgba(59, 130, 246, 0.2)',
        tension: 0.3,
        pointRadius: 0,
      },
    ],
  }

  const options = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { display: false },
      title: { display: true, text: title },
    },
    scales: {
      y: { beginAtZero: true },
    },
  }

  return (
    <div className="rounded-xl border border-gray-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-800">
      <div className="h-48 w-full sm:h-64">
        <Line data={data} options={options} />
      </div>
    </div>
  )
}
