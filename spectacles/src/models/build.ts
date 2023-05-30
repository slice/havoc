export enum Branch {
  Development = "development",
  Canary = "canary",
  PTB = "ptb",
  Stable = "stable",
}

/** Discord application branches that have an (accessible) frontend. */
export const appBranches: Exclude<Branch, Branch.Development>[] = [
  Branch.Canary,
  Branch.PTB,
  Branch.Stable,
];

/** A Discord frontend build. */
export type Build = {
  id: string;
  number: number;
};

/** An instance of a build being detected on a branch at a certain point in time. */
export type DetectedBuild = Build & { branch: Branch; detectedAt: Date };

export function humanFriendlyBranchName(branch: Branch): string {
  switch (branch) {
    case Branch.Canary:
      return "Canary";
    case Branch.PTB:
      return "PTB";
    case Branch.Stable:
      return "Stable";
    case Branch.Development:
      return "Development";
    default:
      return branch;
  }
}
