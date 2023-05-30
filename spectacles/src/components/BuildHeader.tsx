import classNames from "classnames";
import { formatDistance } from "date-fns";
import { Branch, DetectedBuild, humanFriendlyBranchName } from "@/models/build";
import styles from "./BuildHeader.module.css";

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  weekday: "short",
  month: "short",
  day: "numeric",
  year: "numeric",
});

export default function BuildHeader(props: {
  branch: Branch | "dual";
  build: DetectedBuild;
}) {
  const date = new Date(props.build.detectedAt);
  const branch: React.ReactNode =
    props.branch === "dual" ? (
      <>
        <div className={classNames(styles.buildBranchDual, "build-canary")}>
          Canary
        </div>
        <div className={classNames(styles.buildBranchDual, "build-ptb")}>
          <span className={styles.unemphasized}>{"& "}</span>PTB
        </div>
      </>
    ) : (
      humanFriendlyBranchName(props.branch as Branch)
    );
  const ago = formatDistance(date, new Date());

  return (
    <div
      className={classNames(
        styles.buildHeader,
        props.branch === "dual" ? styles.buildDual : null,
        `build-${props.branch}`
      )}
    >
      <div className={styles.buildName}>
        <span className={styles.buildBranch}>{branch}</span>{" "}
        <span className={styles.buildNumber}>{props.build.number}</span>
      </div>
      <div className={styles.buildMetadata}>
        <div className={styles.buildTimestamps}>
          <div>
            detected <strong>{ago}</strong> ago
          </div>
          <div
            className={styles.buildAbsoluteTimestamp}
            suppressHydrationWarning
          >
            {dateFormatter.format(date)}
          </div>
        </div>
      </div>
    </div>
  );
}
