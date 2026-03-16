import * as vscode from "vscode";
import { EnvSafeTreeProvider, EnvVariable } from "./provider";

let treeProvider: EnvSafeTreeProvider;

export function activate(context: vscode.ExtensionContext): void {
  treeProvider = new EnvSafeTreeProvider();

  const treeView = vscode.window.createTreeView("envsafeVariables", {
    treeDataProvider: treeProvider,
  });
  context.subscriptions.push(treeView);

  context.subscriptions.push(
    vscode.commands.registerCommand("envsafe.refresh", () => {
      treeProvider.refresh();
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("envsafe.addVariable", async () => {
      const env = await pickEnvironment();
      if (!env) {
        return;
      }

      const key = await vscode.window.showInputBox({
        prompt: "Variable name",
        placeHolder: "MY_SECRET",
      });
      if (!key) {
        return;
      }

      const value = await vscode.window.showInputBox({
        prompt: `Value for ${key}`,
        password: true,
      });
      if (value === undefined) {
        return;
      }

      runEnvsafe(`set ${key} "${value}" --env ${env}`);
      treeProvider.refresh();
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand(
      "envsafe.editVariable",
      async (item?: EnvVariable) => {
        const key = item?.key ?? (await vscode.window.showInputBox({ prompt: "Variable name to edit" }));
        if (!key) {
          return;
        }

        const env = item?.environment ?? (await pickEnvironment());
        if (!env) {
          return;
        }

        const value = await vscode.window.showInputBox({
          prompt: `New value for ${key}`,
          password: true,
        });
        if (value === undefined) {
          return;
        }

        runEnvsafe(`set ${key} "${value}" --env ${env}`);
        treeProvider.refresh();
      }
    )
  );

  context.subscriptions.push(
    vscode.commands.registerCommand(
      "envsafe.deleteVariable",
      async (item?: EnvVariable) => {
        const key = item?.key ?? (await vscode.window.showInputBox({ prompt: "Variable name to delete" }));
        if (!key) {
          return;
        }

        const env = item?.environment ?? (await pickEnvironment());
        if (!env) {
          return;
        }

        const confirm = await vscode.window.showWarningMessage(
          `Delete variable "${key}" from environment "${env}"?`,
          { modal: true },
          "Delete"
        );
        if (confirm !== "Delete") {
          return;
        }

        runEnvsafe(`rm ${key} --env ${env}`);
        treeProvider.refresh();
      }
    )
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("envsafe.switchEnv", async () => {
      const env = await pickEnvironment();
      if (!env) {
        return;
      }
      treeProvider.setActiveEnvironment(env);
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("envsafe.exportDotenv", async () => {
      const env = await pickEnvironment();
      if (!env) {
        return;
      }

      const uri = await vscode.window.showSaveDialogOptions({
        defaultUri: vscode.Uri.file(".env"),
        filters: { "Environment Files": ["env"] },
      });
      if (!uri) {
        return;
      }

      try {
        const output = runEnvsafe(`export --format dotenv --env ${env}`);
        if (output !== null) {
          const encoder = new TextEncoder();
          await vscode.workspace.fs.writeFile(
            uri as vscode.Uri,
            encoder.encode(output)
          );
          vscode.window.showInformationMessage(`Exported ${env} environment to ${(uri as vscode.Uri).fsPath}`);
        }
      } catch {
        vscode.window.showErrorMessage("Failed to export environment variables.");
      }
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("envsafe.scan", () => {
      const output = runEnvsafe("scan");
      if (output !== null) {
        const channel = vscode.window.createOutputChannel("EnvSafe Scan");
        channel.appendLine(output);
        channel.show();
      }
    })
  );

  // Warn when opening .env files
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((doc) => {
      const fileName = doc.fileName;
      if (/\.env(\.|$)/i.test(fileName) && !fileName.includes(".envsafe")) {
        vscode.window.showWarningMessage(
          "You are editing a .env file. Consider using EnvSafe to manage secrets securely.",
          "Open EnvSafe"
        ).then((choice) => {
          if (choice === "Open EnvSafe") {
            vscode.commands.executeCommand("workbench.view.extension.envsafe");
          }
        });
      }
    })
  );

  vscode.window.showInformationMessage("EnvSafe extension activated.");
}

export function deactivate(): void {
  // Nothing to clean up
}

function runEnvsafe(args: string): string | null {
  const cp = require("child_process") as typeof import("child_process");
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!workspaceRoot) {
    vscode.window.showErrorMessage("No workspace folder open.");
    return null;
  }

  try {
    const result = cp.execSync(`envsafe ${args}`, {
      cwd: workspaceRoot,
      encoding: "utf-8",
      timeout: 10000,
    });
    return result.trim();
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    vscode.window.showErrorMessage(`EnvSafe CLI error: ${message}`);
    return null;
  }
}

async function pickEnvironment(): Promise<string | undefined> {
  const cp = require("child_process") as typeof import("child_process");
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!workspaceRoot) {
    vscode.window.showErrorMessage("No workspace folder open.");
    return undefined;
  }

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

    if (envs.length === 0) {
      vscode.window.showWarningMessage("No environments found.");
      return undefined;
    }

    return vscode.window.showQuickPick(envs, {
      placeHolder: "Select an environment",
    });
  } catch {
    vscode.window.showErrorMessage("Failed to list environments. Is the envsafe CLI installed?");
    return undefined;
  }
}

interface ShowSaveDialogOptions {
  defaultUri?: vscode.Uri;
  filters?: Record<string, string[]>;
}

declare module "vscode" {
  namespace window {
    function showSaveDialogOptions(options: ShowSaveDialogOptions): Thenable<vscode.Uri | undefined>;
  }
}
