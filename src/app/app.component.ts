import { ChangeDetectionStrategy, Component, inject } from "@angular/core";
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
  protected readonly connections = inject(ConnectionsFacade).entries;
}
