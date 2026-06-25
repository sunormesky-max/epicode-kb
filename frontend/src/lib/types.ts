// Shared types aligned with backend Rust structs

export type Provenance = 'human' | 'ai' | 'co' | 'conflict'

export type ReviewStatus = 'pending' | 'accepted' | 'rejected' | 'expired'

export type SearchMode = 'semantic' | 'fulltext' | 'hybrid'

export interface Memory {
  id: string
  space_id: string
  content: string
  embedding_model: string
  provenance: Provenance
  provenance_meta?: Record<string, unknown> | null
  trust_level: number
  review_status: ReviewStatus
  parent_conflict_id?: string | null
  last_accessed_at?: number | null
  access_count: number
  created_at: number
  updated_at: number
}

export interface SearchResult {
  memory: Memory
  score: number
  semantic_score?: number
  fulltext_score?: number
  trust_weight: number
  highlight?: string
}

export interface SearchResponse {
  results: SearchResult[]
  total: number
  query_time_ms: number
}

export interface ListMemoriesResponse {
  memories: Memory[]
  total: number
  limit: number
  offset: number
}

export interface RememberResponse {
  id: string
  space_id: string
  content: string
  provenance: Provenance
  trust_level: number
  review_status: ReviewStatus
  embedding_generated: boolean
  created_at: number
}

export interface MemoryCreated {
  id: string
  chunk_index: number
  content_preview: string
}

export interface UploadResponse {
  file_name: string
  file_type: string
  total_chunks: number
  memories_created: MemoryCreated[]
  processing_time_ms: number
}

export interface ApiResponse<T> {
  code: number
  data: T
  message: string
}

export interface CreateMemoryRequest {
  space_id: string
  content: string
  provenance?: Provenance
  trust_level?: number
  provenance_meta?: Record<string, unknown>
  review_status?: ReviewStatus
}

export interface RecallRequest {
  context: string
  space_id: string
  limit?: number
}
