import type { LinkDescriptor, MetaFunction } from '@remix-run/node';
import {
  Link,
  Links,
  LiveReload,
  Meta,
  Outlet,
  Scripts,
  ScrollRestoration,
} from '@remix-run/react';

import rootStyles from '~/styles/global.css';

export function links(): LinkDescriptor[] {
  return [{ rel: 'stylesheet', href: rootStyles }];
}

export const meta: MetaFunction = () => ({
  charset: 'utf-8',
  title: 'spectacles',
  viewport: 'width=device-width,initial-scale=1',
});

export default function App() {
  return (
    <html lang="en">
      <head>
        <Meta />
        <Links />
      </head>
      <body>
        <header className="main-header">
          <div className="brand">spectacles</div>
          <Link to="/">home</Link>
          <Link to="/builds">builds</Link>
          <Link to="/manage">manage</Link>
        </header>
        <main>
          <Outlet />
        </main>
        <ScrollRestoration />
        <Scripts />
        <LiveReload />
      </body>
    </html>
  );
}
