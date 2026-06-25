// tRPC router definition (vanilla mode).
// Defines the API surface using zod schemas for input validation.

import { z } from 'zod'

// ============================================================
// Zod Schemas
// ============================================================

export const provenanceSchema = z.enum(['human', 'ai', 'co', 'conflict'])
export const reviewStatusSchema = z.enum(['pending', 'accepted', 'rejected', 'expired'])
export const searchModeSchema = z.enum(['semantic', 'fulltext', 'hybrid'])

export const createMemorySchema = z.object({
  space_id: z.string().min(1),
  content: z.string().min(1),
  provenance: provenanceSchema.default('human'),
  trust_level: z.number().min(0).max(1).optional(),
  provenance_meta: z.record(z.unknown()).optional(),
  review_status: reviewStatusSchema.optional(),
})

export const searchSchema = z.object({
  q: z.string().min(1),
  space_id: z.string().min(1),
  mode: searchModeSchema.default('hybrid'),
  min_trust: z.number().min(0).max(1).optional(),
  provenance: z.string().optional(),
  review_status: z.string().optional(),
  limit: z.number().min(1).max(100).default(20),
  offset: z.number().min(0).default(0),
})

export const recallSchema = z.object({
  context: z.string().min(1),
  space_id: z.string().min(1),
  limit: z.number().min(1).max(50).default(10),
})

export const updateTrustSchema = z.object({
  trust_level: z.number().min(0).max(1),
  reason: z.string().optional(),
})

// ============================================================
// Router type (for type-safe client usage)
// ============================================================

export type Router = {
  memory: {
    remember: typeof createMemorySchema
    updateTrust: typeof updateTrustSchema
  }
  search: {
    search: typeof searchSchema
    recall: typeof recallSchema
  }
}

export const routerDefinition: Router = {
  memory: {
    remember: createMemorySchema,
    updateTrust: updateTrustSchema,
  },
  search: {
    search: searchSchema,
    recall: recallSchema,
  },
}
