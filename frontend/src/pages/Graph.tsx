export default function Graph() {
  return (
    <div className="max-w-3xl mx-auto">
      <h1 className="text-2xl font-bold text-gray-900 mb-1">Knowledge Graph</h1>
      <p className="text-gray-500 mb-8">Visualize memory relationships and connections.</p>

      <div className="bg-white rounded-xl border border-gray-200 p-12 text-center">
        <div className="text-5xl mb-4">🕸️</div>
        <h2 className="text-xl font-semibold text-gray-700 mb-2">Coming in Sprint 2</h2>
        <p className="text-gray-500 max-w-md mx-auto">
          The interactive knowledge graph visualization will be available in Sprint 2,
          powered by Cytoscape.js. It will display memory relationships including
          links, merges, conflicts, and summaries.
        </p>
        <div className="mt-6 inline-flex items-center gap-2 px-4 py-2 bg-yellow-50 rounded-lg">
          <span className="text-yellow-600 text-sm font-medium">🚧 Under Development</span>
        </div>
      </div>
    </div>
  )
}
