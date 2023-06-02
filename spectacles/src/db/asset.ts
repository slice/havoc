import db from "@/db";
import { Asset } from "@/models/asset";

/** Fetch all assets associated with a particular build. */
export async function fetchBuildAssets(buildID: string): Promise<Asset[]> {
  let result = await db.query(
    `
      SELECT *
      FROM build_assets
      JOIN assets ON assets.name = build_assets.asset_name
      WHERE build_id = $1
    `,
    [buildID]
  );

  return result.rows.map((row) => ({
    name: row.name,
    surface: row.surface,
    surfaceScriptType: row.surface_script_type,
    scriptChunkId: row.script_chunk_id,
  }));
}
