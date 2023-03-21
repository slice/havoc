import { format, formatDistance } from 'date-fns';
import { Branch, Build, humanFriendlyBranchName } from '~/build';

export default function BuildHeader(props: { branch: Branch; build: Build }) {
  const date = new Date(props.build.detectedAt);
  const ago = formatDistance(date, new Date());
  const absolute = format(date, 'E, MMM d y');

  return (
    <div className={`build-header build-${props.branch}`}>
      <div className="build-name">
        <span className="build-branch">
          {humanFriendlyBranchName(props.branch)}
        </span>{' '}
        <span className="build-number">{props.build.buildNumber}</span>
      </div>
      <div className="build-metadata">
        <div className="build-timestamps">
          <div className="build-relative-timestamp">
            detected <strong>{ago}</strong> ago
          </div>
          <div className="build-absolute-timestamp">{absolute}</div>
        </div>
      </div>
    </div>
  );
}
