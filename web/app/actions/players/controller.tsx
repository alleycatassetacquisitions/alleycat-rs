import { createController } from "remix/router";

import { routes } from "../../routes.ts";
import { PlayersPage } from "./show-page.tsx"

export default createController(routes.players, {
  actions: {
    show(context) {
      return context.render(<PlayersPage />)
    },
  },
});
