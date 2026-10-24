/**
 * Supported AI Models Configuration and Registry
 */

export interface ModelMetadata {
  id: string;
  name: string;
  provider: string;
  contextLimit: string;
  description: string;
}

export const SUPPORTED_MODELS: ModelMetadata[] = [
  {
    id: '@cf/google/gemma-4-26b-a4b-it',
    name: 'Gemma 4 26B A4B',
    provider: 'Cloudflare Workers AI',
    contextLimit: '128K tokens',
    description: 'Google Gemma 4 mixture-of-experts model (26B total, 4B active). Native reasoning, tool calling, and streaming.',
  },
  {
    id: '@cf/qwen/qwen3.8-27b',
    name: 'Qwen 3.8 27B',
    provider: 'Cloudflare Workers AI',
    contextLimit: '32K tokens',
    description: 'Qwen 3.8 27B dense model. High speed reasoning and function calling.',
  },
];

export const DEFAULT_MODEL_ID = '@cf/google/gemma-4-26b-a4b-it';

export const AVAILABLE_MODEL_IDS = SUPPORTED_MODELS.map((m) => m.id);

export function getModelMetadata(modelId?: string): ModelMetadata {
  const target = modelId || DEFAULT_MODEL_ID;
  const found = SUPPORTED_MODELS.find((m) => m.id === target);
  if (found) return found;

  return {
    id: target,
    name: target.split('/').pop() || target,
    provider: 'Cloudflare Workers AI',
    contextLimit: 'Unknown',
    description: 'Cloudflare Workers AI model',
  };
}

export function getModelDisplayName(modelId?: string): string {
  return getModelMetadata(modelId).name;
}
