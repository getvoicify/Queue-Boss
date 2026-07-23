import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from "@angular/core";
import { ConnectionsFacade } from "./core/facades/connections.facade";
import { ShellComponent } from "./shell/shell.component";

@Component({
  selector: "app-root",
  imports: [ShellComponent],
  templateUrl: "./app.component.html",
  changeDetection: ChangeDetectionStrategy.OnPush,
  styleUrl: "./app.component.css",
})
export class AppComponent {
  readonly #connections = inject(ConnectionsFacade);
  protected readonly status = computed(
    () =>
      this.#connections.statusFor(this.#connections.activeConnectionId())
        ?.status ?? "idle",
  );
}
