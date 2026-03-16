import * as vscode from "vscode";

export class EnvVariable extends vscode.TreeItem {
  public readonly key: string;
  public readonly environment: string;

  constructor(key: string, maskedValue: string, environment: string) {
    super(key, vscode.TreeItemCollapsibleState.None);
    this.key = key;
    this.environment = environment;
    this.description = maskedValue;
    this.tooltip = `${key} (${environment}) - click to reveal`;
    this.contextValue = "envVariable";
    this.iconPath = new vscode.ThemeIcon("key");

    this.command = {
      command: "envsafe.editVariable",
      title: "Edit Variable",
      arguments: [this],
    };
  }
}

class EnvironmentGroup extends vscode.TreeItem {
  constructor(
    public readonly envName: string,
    public readonly variables: EnvVariable[]
  ) {
    super(envName, vscode.TreeItemCollapsibleState.Expanded);
    this.contextValue = "environment";
    this.iconPath = new vscode.ThemeIcon("server-environment");
  }
}

type TreeElement = EnvironmentGroup | EnvVariable;

export class EnvSafeTreeProvider implements vscode.TreeDataProvider<TreeElement> {
  private _onDidChangeTreeData = new vscode.EventEmitter<TreeElement | undefined | void>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  private activeEnvironment: string | undefined;

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  setActiveEnvironment(env: string): void {
    this.activeEnvironment = env;
    this.refresh();
  }

  getTreeItem(element: TreeElement): vscode.TreeItem {
    return element;
  }

  getChildren(element?: TreeElement): vscode.ProviderResult<TreeElement[]> {
    if (!vscode.workspace.workspaceFolders) {
      return [];
    }

    const workspaceRoot = vscode.workspace.workspaceFolders[0].uri.fsPath;

    if (!element) {
      // Root level: show environments
      return this.getEnvironments(workspaceRoot);
    }

    if (element instanceof EnvironmentGroup) {
      // Environment level: show variables
      return this.getVariables(workspaceRoot, element.envName);
    }

    return [];
  }

  private getEnvironments(workspaceRoot: string): TreeElement[] {
    const cp = require("child_process") as typeof import("child_process");

    try {
      const output = cp.execSync("envsafe envs", {
        cwd: workspaceRoot,
        encoding: "utf-8",
        timeout: 10000,
      });

      const envs = output
        .trim()
        .split("\n")
        .map((e: string) => e.trim())
        .filter((e: string) => e.length > 0);

      // If an active environment is set, show only that one
      if (this.activeEnvironment && envs.includes(this.activeEnvironment)) {
        const vars = this.getVariables(workspaceRoot, this.activeEnvironment);
        return [new EnvironmentGroup(this.activeEnvironment, vars)];
      }

      return envs.map((env: string) => {
        const vars = this.getVariables(workspaceRoot, env);
        return new EnvironmentGroup(env, vars);
      });
    } catch {
      vscode.window.showErrorMessage(
        "Failed to list environments. Is the envsafe CLI installed?"
      );
      return [];
    }
  }

  private getVariables(workspaceRoot: string, env: string): EnvVariable[] {
    const cp = require("child_process") as typeof import("child_process");

    try {
      const output = cp.execSync(`envsafe export --format json --env ${env}`, {
        cwd: workspaceRoot,
        encoding: "utf-8",
        timeout: 10000,
      });

      const variables: Record<string, string> = JSON.parse(output);

      return Object.entries(variables).map(([key, value]) => {
        const masked = maskValue(value);
        return new EnvVariable(key, masked, env);
      });
    } catch {
      return [];
    }
  }
}

function maskValue(value: string): string {
  if (value.length <= 4) {
    return "****";
  }
  return value.substring(0, 2) + "*".repeat(Math.min(value.length - 2, 8)) + value.substring(value.length - 2);
}
