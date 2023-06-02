export interface Asset {
  name: string;
  surface: boolean;
  surfaceScriptType:
    | "chunkloader"
    | "classes"
    | "vendor"
    | "entrypoint"
    | undefined;
  scriptChunkId: number | undefined;
}
