import { json, LinksFunction } from '@remix-run/node';
import { useLoaderData } from '@remix-run/react';
import { appBranches, Branch, Build } from '~/build';
import { latestBuildOnBranch } from '~/build.server';
import BuildHeader from '~/components/BuildHeader';
import buildHeaderStyles from '~/components/BuildHeader.css';

export const links: LinksFunction = () => [
  { rel: 'stylesheet', href: buildHeaderStyles },
];

export async function loader() {
  const latest = Object.fromEntries(
    await Promise.all(
      appBranches.map((branch) =>
        latestBuildOnBranch(branch).then((result) => [branch, result])
      )
    )
  ) as { [branch in Exclude<Branch, Branch.Development>]: Build };

  return json({ latest });
}

export default function Index() {
  const data = useLoaderData<typeof loader>();

  return (
    <section className="latest-builds">
      <h1>Latest Builds {/*<span className="emphasis-badge">live</span>*/}</h1>
      {appBranches.map((branch) => (
        <BuildHeader
          branch={branch}
          build={{
            ...data.latest[branch],
            detectedAt: new Date(data.latest[branch].detectedAt),
          }}
          key={branch}
        />
      ))}
    </section>
  );
}
