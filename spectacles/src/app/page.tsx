import React from "react";
import { appBranches, Branch, DetectedBuild } from "@/models/build";
import { latestBuildOnBranch } from "@/db/build";
import BuildHeader from "@/components/BuildHeader";
import db from "@/db";
import WrappingBuildsList from "@/components/WrappingBuildsList";
import styles from "./page.module.css";

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  weekday: "short",
  month: "short",
  day: "numeric",
});

export default async function Index() {
  let latestBuildsEntries = appBranches.map((branch) => [
    branch,
    latestBuildOnBranch(branch),
  ]);
  let historicalStatement = db.prepare(`
    SELECT branch, build_number, build_id, detected_at
    FROM detections
    WHERE detected_at > strftime('%s', 'now', '-7 days')
    ORDER BY detected_at DESC
  `);
  let historicalBuildsRows = historicalStatement.all() as any[];

  // When will TypeScript properly support this :pensive:
  let latestBuilds = Object.fromEntries(latestBuildsEntries) as {
    [branch in Exclude<Branch, Branch.Development>]: DetectedBuild;
  };

  let historical: DetectedBuild[] = historicalBuildsRows.map((row) => ({
    branch: row.branch,
    number: row.build_number,
    id: row.build_id,
    detectedAt: new Date(row.detected_at * 1000),
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
          <BuildHeader
            branch="dual"
            build={latest.canary}
            detectedAt={latest.canary.detectedAt}
          />
        ) : (
          <>
            <BuildHeader
              branch={Branch.Canary}
              build={latest.canary}
              detectedAt={latest.canary.detectedAt}
            />
            <BuildHeader
              branch={Branch.PTB}
              build={latest.ptb}
              detectedAt={latest.ptb.detectedAt}
            />
          </>
        )}
        <BuildHeader
          branch={Branch.Stable}
          build={latest.stable}
          detectedAt={latest.stable.detectedAt}
        />
      </section>
      <section className="historical-builds">
        <h2 style={{ marginTop: "2rem" }}>Last 7 Days of Builds</h2>
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
