"use client";

import React, { ComponentProps } from "react";
import styles from "./Copier.module.css";
import classNames from "classnames";

export default function Copier({
  content,
  children,
}: {
  content: string;
  children: (
    props: Pick<React.HTMLAttributes<HTMLElement>, "className"> &
      Pick<React.HTMLAttributes<HTMLElement>, "onClick">
  ) => JSX.Element;
}) {
  return children({
    className: styles.copier,
    onClick(event: React.MouseEvent) {
      if (event.shiftKey || event.metaKey || event.altKey) return;
      event.preventDefault();
      navigator.clipboard.writeText(content);
    },
  });
}

export function CopiableLink({
  href,
  children,
  ...passedProps
}: ComponentProps<"a"> & {
  href: string;
  children: React.ReactNode;
}) {
  return (
    <Copier content={href}>
      {(copierProps) => (
        <a
          href={href}
          {...copierProps}
          {...passedProps}
          className={classNames(copierProps.className, passedProps.className)}
        >
          {children}
        </a>
      )}
    </Copier>
  );
}

export function CopiableCodeBlock({
  children,
  ...passedProps
}: ComponentProps<"pre"> & { children: string }) {
  return (
    <Copier content={children}>
      {(copierProps) => (
        <pre
          {...copierProps}
          {...passedProps}
          className={classNames(copierProps.className, passedProps.className)}
        >
          <code>{children}</code>
        </pre>
      )}
    </Copier>
  );
}
