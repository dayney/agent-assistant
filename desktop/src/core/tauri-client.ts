import { invoke } from '@tauri-apps/api/core';
import type { CoreClient } from './client';
import type { ApplyPreview, WorkspaceSnapshot } from './model';

export class TauriCoreClient implements CoreClient {
    async getSnapshot(): Promise<WorkspaceSnapshot> {
        try {
            return await invoke<WorkspaceSnapshot>('get_workspace_snapshot');
        } catch (error) {
            throw new Error(String(error));
        }
    }

    async previewApply(): Promise<ApplyPreview> {
        try {
            return await invoke<ApplyPreview>('preview_apply');
        } catch (error) {
            throw new Error(String(error));
        }
    }
}
