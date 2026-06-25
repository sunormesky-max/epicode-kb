export default function TrustIndicator({ trust }: { trust: number }) {
  const percentage = Math.round(trust * 100)
  const color =
    trust >= 0.8 ? 'bg-green-500' : trust >= 0.5 ? 'bg-yellow-500' : 'bg-red-500'

  return (
    <div className="flex items-center gap-2">
      <div className="flex-1 h-2 bg-gray-200 rounded-full overflow-hidden min-w-[60px]">
        <div
          className={`h-full ${color} transition-all duration-300`}
          style={{ width: `${percentage}%` }}
        />
      </div>
      <span className="text-xs font-medium text-gray-600 tabular-nums">{percentage}%</span>
    </div>
  )
}
