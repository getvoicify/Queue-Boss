import { ChangeDetectionStrategy, Component, inject } from "@angular/core";
import { ConnectionFacade } from "./core/facades/connection.facade";
import { ShellComponent } from "./shell/shell.component";

@Component({
  selector: "app-root",
  imports: [ShellComponent],
  templateUrl: "./app.component.html",
  changeDetection: ChangeDetectionStrategy.OnPush,
  styleUrl: "./app.component.css",
})
export class AppComponent {
  protected readonly status = inject(ConnectionFacade).status;
}
