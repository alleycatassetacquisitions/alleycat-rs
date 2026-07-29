import type { Handle } from "remix/ui";
import { css } from "remix/ui";

import { Document } from "../../ui/document.tsx";

export function PlayersPage() {
  return () => {
    return (
      <Document title="Players">
        <main
          mix={css({
                 maxWidth: "44rem",
                 margin: "0 auto",
                 padding: "4rem 1.5rem",
               })}
        >
          <h1>Players</h1>
          <p>display players right here</p>
        </main>
      </Document>
    );
  };
}
