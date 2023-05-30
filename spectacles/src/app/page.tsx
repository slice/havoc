import React from "react";
import { appBranches, Branch, DetectedBuild } from "@/models/build";
import { latestBuildOnBranch } from "@/db/build";
import BuildHeader from "@/components/BuildHeader";
import pg from "@/db";
import WrappingBuildsList from "@/components/WrappingBuildsList";
import styles from "./page.module.css";

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  weekday: "short",
  month: "short",
  day: "numeric",
});

export default async function Index() {
  const [latestBuildsEntries, historicalBuildsRows] = await Promise.all([
    Promise.all(
      appBranches.map((branch) =>
        latestBuildOnBranch(branch).then((result) => [branch, result])
      )
    ),
    pg.query(`
        SELECT branch, build_number, build_id, detected_at
        FROM detections
        WHERE detected_at > (current_timestamp - INTERVAL '7 days')
        ORDER BY detected_at DESC
      `),
  ]);

  // When will TypeScript properly support this :pensive:
  let latestBuilds = Object.fromEntries(latestBuildsEntries) as {
    [branch in Exclude<Branch, Branch.Development>]: DetectedBuild;
  };

  let historical: DetectedBuild[] = historicalBuildsRows.rows.map((row) => ({
    branch: row.branch,
    number: row.build_number,
    id: row.build_id,
    detectedAt: row.detected_at,
  }));

  const latest = Object.fromEntries(
    Object.entries(latestBuilds).map(([branch, build]) => [branch, build])
  ) as { [branch in Branch]: DetectedBuild };

  let today = historical[0].detectedAt;
  let builds: DetectedBuild[] = [];
  let calendarized: { day: Date; builds: DetectedBuild[] }[] = [];
  for (const entry of historical) {
    const detectedAt = entry.detectedAt;
    if (
      detectedAt.getDay() !== today.getDay() ||
      detectedAt.getMonth() !== today.getMonth() ||
      detectedAt.getFullYear() !== today.getFullYear()
    ) {
      calendarized.push({ day: today, builds });
      today = detectedAt;
      builds = [];
    }
    builds.push(entry);
  }
  calendarized.push({ day: today, builds });

  return (
    <>
      <section className="latest-builds">
        <h2>
          Latest Builds {/*<span className="emphasis-badge">live</span>*/}
        </h2>
        {latest.ptb.number === latest.canary.number ? (
          <BuildHeader branch="dual" build={latest.canary} />
        ) : (
          <>
            <BuildHeader branch={Branch.Canary} build={latest.canary} />
            <BuildHeader branch={Branch.PTB} build={latest.ptb} />
          </>
        )}
        <BuildHeader branch={Branch.Stable} build={latest.stable} />
      </section>
      <section className="historical-builds">
        <h2 style={{ marginTop: "2rem" }}>Recent Builds</h2>
        <div className={styles.historicalBuildsCalendar}>
          {calendarized.map(({ day, builds }) => (
            <React.Fragment key={day.toUTCString()}>
              <h3 title={day.toLocaleString()} suppressHydrationWarning>
                {dateFormatter.format(day)}
                <br />
                <small>
                  {builds.length} build{builds.length === 1 ? "" : "s"}
                </small>
              </h3>
              <WrappingBuildsList latestBuilds={latest} builds={builds} />
            </React.Fragment>
          ))}
        </div>
      </section>
    </>
  );
}

export const dynamic = "force-dynamic";
