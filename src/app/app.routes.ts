import type { Routes } from "@angular/router";
import { LifecycleComponent } from "./features/lifecycle/lifecycle.component";
import { OverviewContainerComponent } from "./features/overview/overview-container.component";

export const routes: Routes = [
  {
    path: "overview",
    component: OverviewContainerComponent,
    title: "Overview",
  },
  { path: "lifecycle", component: LifecycleComponent, title: "Lifecycle" },
  { path: "", pathMatch: "full", redirectTo: "overview" },
];
