import type { Routes } from "@angular/router";
import { LifecycleComponent } from "./features/lifecycle/lifecycle.component";
import { OverviewComponent } from "./features/overview/overview.component";

export const routes: Routes = [
  { path: "overview", component: OverviewComponent, title: "Overview" },
  { path: "lifecycle", component: LifecycleComponent, title: "Lifecycle" },
  { path: "", pathMatch: "full", redirectTo: "overview" },
];
