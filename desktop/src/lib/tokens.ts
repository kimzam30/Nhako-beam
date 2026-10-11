// Re-exports the generated design tokens so app code imports from one place.
// Source of truth: design/tokens/tokens.json (run `npm run tokens` after editing it).
export * from '../../../design/tokens/build/tokens.js';
import { spring } from '../../../design/tokens/build/tokens.js';

export type SpringName = keyof typeof spring;
