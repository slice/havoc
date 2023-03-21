import React from 'react';
import { json, LinksFunction, SerializeFrom } from '@remix-run/node';
import { useLoaderData } from '@remix-run/react';
import { appBranches, Branch, DetectedBuild } from '~/build';
import { latestBuildOnBranch } from '~/build.server';
import BuildHeader from '~/components/BuildHeader';
import buildHeaderStyles from '~/components/BuildHeader.css';
import historicalBuildsStyles from '~/styles/historicalBuilds.css';
import WrappingBuildsListStyles from '~/components/WrappingBuildsList.css';
import pg from '~/db.server';
import { format } from 'date-fns';
import WrappingBuildsList from '~/components/WrappingBuildsList';

export const links: LinksFunction = () => [
  { rel: 'stylesheet', href: buildHeaderStyles },
  { rel: 'stylesheet', href: historicalBuildsStyles },
  { rel: 'stylesheet', href: WrappingBuildsListStyles },
];

export async function loader() {
  const [latestBuildsEntries, historicalBuildsRows] = await Promise.all([
    Promise.all(
      appBranches.map((branch) =>
        latestBuildOnBranch(branch).then((result) => [branch, result])
      )
    ),
    pg.query(`
        SELECT dbob.branch, db.build_number, db.build_id, dbob.detected_at
        FROM detected_builds_on_branches dbob
        INNER JOIN detected_builds db ON db.build_id = dbob.build_id
        WHERE dbob.detected_at > (current_timestamp - interval '7 days')
        ORDER BY dbob.detected_at DESC
      `),
  ]);

  // When will TypeScript properly support this :pensive:
  let latestBuilds = Object.fromEntries(latestBuildsEntries) as {
    [branch in Exclude<Branch, Branch.Development>]: DetectedBuild;
  };

  let historicalBuilds: DetectedBuild[] = historicalBuildsRows.rows.map(
    (row) => ({
      branch: row.branch,
      number: row.build_number,
      id: row.build_id,
      detectedAt: row.detected_at,
    })
  );

  return json({ latest: latestBuilds, historical: historicalBuilds });
}

function deserializeBuild(
  // A bit of a hack to access the serialized type of `Build`.
  build: SerializeFrom<typeof loader>['latest']['canary']
): DetectedBuild {
  return { ...build, detectedAt: new Date(build.detectedAt) };
}

export default function Index() {
  const data = useLoaderData<typeof loader>();

  const latest = Object.fromEntries(
    Object.entries(data.latest).map(([branch, build]) => [
      branch,
      deserializeBuild(build),
    ])
  ) as { [branch in Branch]: DetectedBuild };
  const historical: DetectedBuild[] = data.historical.map(deserializeBuild);

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
        <h2 style={{ marginTop: '2rem' }}>Recent Builds</h2>
        <div className="historical-builds-calendar">
          {calendarized.map(({ day, builds }) => (
            <React.Fragment key={day.toUTCString()}>
              <h3>
                {format(day, 'E, MMM d')}
                <br />
                <small>
                  {builds.length} build{builds.length === 1 ? '' : 's'}
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
