import classNames from "classnames";
import { Branch, DetectedBuild } from "@/models/build";
import { collapseBranches } from "@/models/collapsing";
import styles from "./WrappingBuildsList.module.css";
import Link from "next/link";

const timeFormatter = new Intl.DateTimeFormat(undefined, {
  timeStyle: "long",
});

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
    <div className={styles.wrappingBuildsList}>
      {collapsedBuilds.map((build) => {
        const isCollapsed = build.branch === "collapsed";

        const isCurrent = isCollapsed
          ? latestVersion(Branch.PTB) === build.number &&
            latestVersion(Branch.Canary) === build.number
          : latestVersion(build.branch) === build.number;

        return (
          <Link
            href={`/build/${build.number}`}
            className={classNames(
              styles.build,
              `build-${build.branch}`,
              build.branch === "collapsed" ? styles.buildCollapsed : null,
              isCurrent && styles.buildCurrent
            )}
            title={timeFormatter.format(build.detectedAt)}
            suppressHydrationWarning
            key={build.detectedAt.getTime()}
          >
            {build.number}
          </Link>
        );
      })}
    </div>
  );
}
