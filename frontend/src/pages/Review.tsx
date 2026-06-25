export default function Review() {
  return (
    <div className="max-w-3xl mx-auto">
      <h1 className="text-2xl font-bold text-gray-900 mb-1">Review Queue</h1>
      <p className="text-gray-500 mb-8">Review and approve AI-proposed memories.</p>

      <div className="bg-white rounded-xl border border-gray-200 p-12 text-center">
        <div className="text-5xl mb-4">📋</div>
        <h2 className="text-xl font-semibold text-gray-700 mb-2">Coming in Sprint 3</h2>
        <p className="text-gray-500 max-w-md mx-auto">
          The AI proposal review queue will be available in Sprint 3.
          This feature includes AI-generated merge, link, summarize, and conflict
          proposals for human review and approval.
        </p>
        <div className="mt-6 inline-flex items-center gap-2 px-4 py-2 bg-yellow-50 rounded-lg">
          <span className="text-yellow-600 text-sm font-medium">🚧 Under Development</span>
        </div>
      </div>
    </div>
  )
}
