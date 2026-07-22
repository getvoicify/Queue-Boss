import { ChangeDetectionStrategy, Component } from "@angular/core";
import { ShellComponent } from "./shell/shell.component";

@Component({
  selector: "app-root",
  imports: [ShellComponent],
  templateUrl: "./app.component.html",
  changeDetection: ChangeDetectionStrategy.OnPush,
  styleUrl: "./app.component.css",
})
export class AppComponent {}
