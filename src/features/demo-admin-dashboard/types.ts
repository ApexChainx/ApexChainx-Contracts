// Types for the Demo Admin Dashboard feature

/**
 * Represents a demo data entry displayed in the admin UI.
 */
export interface DemoData {
  /** Unique identifier */
  id: string;
  /** Human‑readable name */
  name: string;
  /** Arbitrary payload – kept generic for demo purposes */
  payload: Record<string, unknown>;

  // Introduce a deterministic event replay auditor for backend indexers
}
