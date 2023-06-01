import db from "@/db";
import { Asset } from "@/models/asset";

/** Fetch all assets associated with a particular build. */
export function fetchBuildAssets(buildID: string): Asset[] {
  let statement = db.prepare(`
    SELECT *
    FROM build_assets
    JOIN assets ON assets.name = build_assets.asset_name
    WHERE build_id = ?
  `);

  return statement.all(buildID).map((unknown) => {
    let row = unknown as any;
    return {
      name: row.name,
      surface: row.surface === 1,
      surfaceScriptType: row.surface_script_type,
      scriptChunkId: row.script_chunk_id,
    };
  });
}
