import classNames from 'classnames';
import { formatDistance } from 'date-fns';
import { Branch, DetectedBuild, humanFriendlyBranchName } from '~/build';

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  weekday: 'short',
  month: 'short',
  day: 'numeric',
  year: 'numeric',
});

export default function BuildHeader(props: {
  branch: Branch | 'dual';
  build: DetectedBuild;
}) {
  const date = new Date(props.build.detectedAt);
  const isDual = props.branch === 'dual';
  const ago = formatDistance(date, new Date());

  return (
    <div className={classNames('build-header', `build-${props.branch}`)}>
      <div className="build-name">
        <span className="build-branch">
          {isDual ? (
            <>
              <div className="build-branch-dual build-canary">Canary</div>
              <div className="build-branch-dual build-ptb">
                <span className="unemphasized">{'& '}</span>PTB
              </div>
            </>
          ) : (
            humanFriendlyBranchName(props.branch as Branch)
          )}
        </span>{' '}
        <span className="build-number">{props.build.number}</span>
      </div>
      <div className="build-metadata">
        <div className="build-timestamps">
          <div className="build-relative-timestamp">
            detected <strong>{ago}</strong> ago
          </div>
          <div className="build-absolute-timestamp" suppressHydrationWarning>
            {dateFormatter.format(date)}
          </div>
        </div>
      </div>
    </div>
  );
}
