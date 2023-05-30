import { Branch, DetectedBuild } from './build';

export type PotentiallyCollapsedBuild =
  | DetectedBuild
  | (Omit<DetectedBuild, 'branch'> & { branch: 'collapsed' });

export function collapseBranches(
  branches: Branch[],
  builds: DetectedBuild[]
): PotentiallyCollapsedBuild[] {
  let collapsedBuilds: PotentiallyCollapsedBuild[] = [];
  const collapsible = (
    first: DetectedBuild | undefined,
    second: DetectedBuild | undefined
  ) =>
    first != null &&
    second != null &&
    branches.includes(first.branch) &&
    branches.includes(second.branch) &&
    first.number == second.number;

  for (let index = 0; index < builds.length; index++) {
    const nextBuild = builds[index + 1];
    const build = builds[index];

    if (collapsible(build, nextBuild)) {
      collapsedBuilds.push({ ...build, branch: 'collapsed' });
      index++;
    } else {
      collapsedBuilds.push(build);
    }
  }

  return collapsedBuilds;
}
