import type { Routes } from "@angular/router";
import { ConnectContainerComponent } from "./features/connect/connect-container.component";
import { JobsContainerComponent } from "./features/jobs/jobs-container.component";
import { LifecycleComponent } from "./features/lifecycle/lifecycle.component";
import { LifecycleHomeContainerComponent } from "./features/lifecycle/lifecycle-home-container.component";
import { OverviewContainerComponent } from "./features/overview/overview-container.component";

export const routes: Routes = [
  { path: "home", component: LifecycleHomeContainerComponent, title: "Home" },
  {
    path: "overview",
    component: OverviewContainerComponent,
    title: "Overview",
  },
  { path: "jobs", component: JobsContainerComponent, title: "Jobs" },
  { path: "lifecycle", component: LifecycleComponent, title: "Lifecycle" },
  { path: "connect", component: ConnectContainerComponent, title: "Connect" },
  { path: "", pathMatch: "full", redirectTo: "home" },
];
