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

export default function BuildDetails({ params }: { params: { id: string } }) {
  let build = fetchBuild(
    /^\d+$/.test(params.id)
      ? { number: parseInt(params.id, 10) }
      : { id: params.id }
  );

  if (build == null) {
    return notFound();
  }

  let latest = latestBuildIDs();

  let appearingAsBranch: React.ComponentProps<typeof BuildHeader>["branch"];
  if (latest.canary === build.id && latest.ptb === build.id) {
    appearingAsBranch = "dual";
  } else {
    appearingAsBranch = Object.entries(latest).find(
      ([_, buildID]) => buildID == build!.id
    )?.[0] as Branch;
  }

  let detections = fetchDetections(build.id);
  let previousBuilds = detections.map((detection) =>
    findPreviousBuild(detection.branch, detection.detectedAt)
  );

  let assets = fetchBuildAssets(build.id).filter((asset) => asset.surface);

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

          <h2>Hash</h2>
          <p>
            <code className={styles.buildHash}>{build.id}</code>
          </p>
          <h2>Assets</h2>
          <ul className={styles.assetList}>
            {assets.map((asset) => (
              <li key={asset.name}>
                <a
                  href={`https://discord.com/assets/${asset.name}`}
                  rel="noreferrer"
                >
                  <code>{asset.name}</code>
                </a>
              </li>
            ))}
          </ul>
        </section>
      </div>
    </>
  );
}

export const dynamic = "force-dynamic";
