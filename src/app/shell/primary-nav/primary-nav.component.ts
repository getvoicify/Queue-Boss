import { ChangeDetectionStrategy, Component } from "@angular/core";
import { RouterLink, RouterLinkActive } from "@angular/router";

@Component({
  selector: "app-primary-nav",
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterLink, RouterLinkActive],
  styleUrl: "./primary-nav.component.css",
  template: `
    <nav class="primary-nav" aria-label="Primary">
      <a
        class="primary-nav__link"
        data-testid="nav-overview"
        routerLink="/overview"
        routerLinkActive="primary-nav__link--active"
        ariaCurrentWhenActive="page"
      >
        Overview
      </a>
      <a
        class="primary-nav__link"
        data-testid="nav-lifecycle"
        routerLink="/lifecycle"
        routerLinkActive="primary-nav__link--active"
        ariaCurrentWhenActive="page"
      >
        Lifecycle
      </a>
      <a
        class="primary-nav__link"
        data-testid="open-connect"
        routerLink="/connect"
        routerLinkActive="primary-nav__link--active"
        ariaCurrentWhenActive="page"
      >
        Connect
      </a>
    </nav>
  `,
})
export class PrimaryNavComponent {}
