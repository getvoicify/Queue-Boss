import { Injectable, inject } from "@angular/core";
import { Title } from "@angular/platform-browser";
import { type RouterStateSnapshot, TitleStrategy } from "@angular/router";

@Injectable()
export class AppTitleStrategy extends TitleStrategy {
  readonly #title = inject(Title);

  override updateTitle(state: RouterStateSnapshot): void {
    const title = this.buildTitle(state);
    this.#title.setTitle(title ? `${title} · Queue Boss` : "Queue Boss");
  }
}
