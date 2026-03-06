import * as os from 'os';
import * as path from 'path';

import * as vscode from 'vscode';
import * as languageClient from 'vscode-languageclient/node';

let client: languageClient.LanguageClient | undefined;

function binaryPath(context: vscode.ExtensionContext): string {
    const ext = os.platform() === 'win32' ? '.exe' : '';
    const binaryPath = context.asAbsolutePath(path.join('bin', `path-server${ext}`));

    return binaryPath;
}

async function openConfiguration(context: vscode.ExtensionContext) {
    const extId = context.extension.id;
    await vscode.commands.executeCommand('workbench.action.openSettings', `@ext:${extId}`);
}

export async function activate(context: vscode.ExtensionContext) {
    const debugMode = context.extensionMode === vscode.ExtensionMode.Development;
    const disposables = [];

    const serverOutputChannel = vscode.window.createOutputChannel("Path Server Language Server", { log: true });
    disposables.push(serverOutputChannel);
    let traceChannel: vscode.OutputChannel | undefined = undefined;
    if (debugMode) {
        traceChannel = vscode.window.createOutputChannel("Path Server Language Server Trace");
        traceChannel.appendLine("Debug mode enabled.");
        disposables.push(traceChannel);
    }

    const serverPath = binaryPath(context);
    const serverExecutable: languageClient.Executable = {
        command: serverPath
    };
    const serverOptions: languageClient.ServerOptions = {
        run: serverExecutable,
        debug: serverExecutable
    };
    const clientOptions: languageClient.LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: '*' },
            { scheme: 'untitled', language: '*' }
        ],
        outputChannel: serverOutputChannel,
        traceOutputChannel: traceChannel,
    };

    client = new languageClient.LanguageClient(
        'path-server',
        'Path Server',
        serverOptions,
        clientOptions
    );
    disposables.push(client);

    const openConfig = vscode.commands.registerCommand("path-server.openConfiguration", async () => await openConfiguration(context));
    disposables.push(openConfig);

    await client.start();
    context.subscriptions.push(vscode.Disposable.from(...disposables));
}

export async function deactivate() {
    await client?.stop();
}