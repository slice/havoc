import React from "react";
import {
  Detection,
  fetchBuild,
  fetchDetections,
  findPreviousBuild,
  latestBuildIDs,
} from "@/db/build";
import BuildHeader from "@/components/BuildHeader";
import { Branch, Build, humanFriendlyBranchName } from "@/models/build";
import { notFound } from "next/navigation";
import styles from "./page.module.css";
import classNames from "classnames";
import { format, formatDistance } from "date-fns";
import Link from "next/link";
import { fetchBuildAssets } from "@/db/asset";
import Copier, { CopiableCodeBlock, CopiableLink } from "@/components/Copier";

function BranchLabel({ branch }: { branch: Branch }) {
  return (
    <strong className={classNames(styles.branchLabel, `build-${branch}`)}>
      {humanFriendlyBranchName(branch)}
    </strong>
  );
}

function DetectionList({
  previousBuilds,
  detections,
}: {
  previousBuilds: (Build | null)[];
  detections: Detection[];
}) {
  return (
    <ol className={styles.detectionList}>
      {detections
        .sort((a, b) => a.detectedAt.getTime() - b.detectedAt.getTime())
        .map((detection, index) => {
          let previousBuild = previousBuilds[index];
          let date =
            index === 0
              ? " on " + format(detection.detectedAt, "MMM d, yyyy")
              : formatDistance(
                  detection.detectedAt,
                  detections[index - 1].detectedAt
                ) + " after";

          return (
            <li key={detection.detectedAt.toISOString()}>
              <div>
                <BranchLabel branch={detection.branch} /> {date}
              </div>
              {previousBuild && (
                <div className={styles.detectionPrev}>
                  (from{" "}
                  <Link href={`/build/${previousBuild.number}`}>
                    {previousBuild.number}
                  </Link>
                  )
                </div>
              )}
            </li>
          );
        })}
    </ol>
  );
}

export default async function BuildDetails({
  params,
}: {
  params: { id: string };
}) {
  let build = await fetchBuild(
    /^\d+$/.test(params.id)
      ? { number: parseInt(params.id, 10) }
      : { id: params.id }
  );

  if (build == null) {
    return notFound();
  }

  let latest = await latestBuildIDs();

  let appearingAsBranch: React.ComponentProps<typeof BuildHeader>["branch"];
  if (latest.canary === build.id && latest.ptb === build.id) {
    appearingAsBranch = "dual";
  } else {
    appearingAsBranch = Object.entries(latest).find(
      ([_, buildID]) => buildID == build!.id
    )?.[0] as Branch;
  }

  let detections = await fetchDetections(build.id);
  let previousBuilds = await Promise.all(
    detections.map((detection) =>
      findPreviousBuild(detection.branch, detection.detectedAt)
    )
  );

  let assets = (await fetchBuildAssets(build.id)).filter(
    (asset) => asset.surface
  );

  return (
    <>
      <BuildHeader
        branch={appearingAsBranch}
        build={{ id: build.id, number: build.number }}
        mergeWithNavigation
        multipleDetections={detections.length > 1}
        detectedAt={detections[0]?.detectedAt}
      />
      <div className={styles.buildInformation}>
        <section>
          <h2>Detections</h2>
          <DetectionList
            detections={detections}
            previousBuilds={previousBuilds}
          />

          <p
            title="Hold down any modifier key to disable this behavior."
            style={{
              textDecoration: "underline dotted hsl(0deg 0% 100% / 50%)",
            }}
          >
            Click things below to copy them.
          </p>

          <h2>Hash</h2>
          <CopiableCodeBlock className={styles.buildHash}>
            {build.id}
          </CopiableCodeBlock>

          <h2>Assets</h2>
          <ul className={styles.assetList}>
            {assets.map((asset) => (
              <li key={asset.name}>
                <CopiableLink href={`https://discord.com/assets/${asset.name}`}>
                  <code>{asset.name}</code>
                </CopiableLink>
              </li>
            ))}
          </ul>
        </section>
        <section>spooky</section>
      </div>
    </>
  );
}

export const dynamic = "force-dynamic";
