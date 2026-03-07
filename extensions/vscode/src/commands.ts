import * as vscode from 'vscode';
import * as languageClient from 'vscode-languageclient/node';

export async function openConfiguration(context: vscode.ExtensionContext) {
    const extId = context.extension.id;
    await vscode.commands.executeCommand('workbench.action.openSettings', `@ext:${extId}`);
}

export async function restartServer(client: languageClient.LanguageClient) {
    await client.stop();
    await client.start();
}