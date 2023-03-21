import classNames from 'classnames';
import { Branch, DetectedBuild } from '~/build';
import { collapseBranches } from '~/collapsing';

export default function WrappingBuildsList(props: {
  builds: DetectedBuild[];
  latestBuilds?: { [branch in Branch]?: DetectedBuild };
}) {
  const collapsedBuilds = collapseBranches(
    [Branch.PTB, Branch.Canary],
    props.builds
  );

  const latestVersion = (branch: Branch): number | undefined =>
    props.latestBuilds?.[branch]?.number;

  return (
    <div className="wrapping-builds-list">
      {collapsedBuilds.map((build) => {
        const isCollapsed = build.branch === 'collapsed';

        const isCurrent = isCollapsed
          ? latestVersion(Branch.PTB) === build.number &&
            latestVersion(Branch.Canary) === build.number
          : latestVersion(build.branch) === build.number;

        return (
          <div
            className={classNames(
              'build',
              `build-${build.branch}`,
              isCurrent && 'build-current'
            )}
            key={build.detectedAt.getTime()}
          >
            {build.number}
          </div>
        );
      })}
    </div>
  );
}
