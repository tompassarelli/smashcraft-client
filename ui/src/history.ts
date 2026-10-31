import { asStore, ingest, type IngestResult } from "./records";
import type { Api } from "./api";

/** Reads every record folder into the stored history and keeps the new matches. */
export async function refreshHistory(api: Pick<Api, "history" | "readRecords" | "saveHistory">): Promise<IngestResult> {
  const [stored, files] = await Promise.all([api.history(), api.readRecords()]);
  const result = ingest(asStore(stored), files);
  if (result.added.length > 0) await api.saveHistory(result.store);
  return result;
}
