import classNames from "classnames";
import { formatDistance } from "date-fns";
import { Branch, Build, humanFriendlyBranchName } from "@/models/build";
import styles from "./BuildHeader.module.css";

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  weekday: "short",
  month: "short",
  day: "numeric",
  year: "numeric",
});

export default function BuildHeader(props: {
  branch?: Branch | "dual";
  build: Build;
  detectedAt?: Date;
  mergeWithNavigation?: boolean;
  multipleDetections?: boolean;
}) {
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

  let ago;
  let date;
  if (props.detectedAt != null) {
    date = new Date(props.detectedAt);
    ago = formatDistance(date, new Date());
  }

  return (
    <div
      className={classNames(
        styles.buildHeader,
        props.branch === "dual" ? styles.buildDual : null,
        props.branch != null ? `build-${props.branch}` : styles.buildPlain,
        props.mergeWithNavigation && styles.mergeWithNavigation
      )}
    >
      <div className={styles.buildName}>
        {branch != null && <span className={styles.buildBranch}>{branch}</span>}{" "}
        <span className={styles.buildNumber}>{props.build.number}</span>
      </div>
      <div className={styles.buildMetadata}>
        {props.detectedAt && (
          <div className={styles.buildTimestamps}>
            <div>
              {props.multipleDetections && "first "}detected{" "}
              <strong>{ago}</strong> ago
            </div>
            <div
              className={styles.buildAbsoluteTimestamp}
              suppressHydrationWarning
            >
              {dateFormatter.format(date)}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
