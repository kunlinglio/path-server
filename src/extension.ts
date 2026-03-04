import * as vscode from 'vscode';
import * as languageClient from 'vscode-languageclient/node';

let client: languageClient.LanguageClient | undefined;

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

    const serverPath = "/Users/lkl/Code/path-server/target/debug/path-server";
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
        traceOutputChannel: traceChannel
    };

    client = new languageClient.LanguageClient(
        'path-server',
        'Path Server',
        serverOptions,
        clientOptions
    );
    disposables.push(client);

    await client.start();
    context.subscriptions.push(vscode.Disposable.from(...disposables));
}

export async function deactivate() {
    await client?.stop();
}