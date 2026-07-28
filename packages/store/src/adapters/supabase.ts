import { GameStore } from '../types/game-store';
import { GameRecord, GameQueryOptions, GameNotFoundError, Unsubscribe } from '../types/game-record';

/**
 * Supabase adapter - PostgreSQL + Realtime + Auth
 * Requires: npm install @supabase/supabase-js
 *
 * Features:
 * - Realtime subscriptions via postgres_changes
 * - Row Level Security aware (games scoped to authenticated user)
 * - Range-based pagination
 */
export class SupabaseGameStore implements GameStore {
  private client: any; // SupabaseClient
  private channels: Map<string, any> = new Map();

  constructor(supabaseUrl: string, supabaseKey: string) {
    try {
      const { createClient } = require('@supabase/supabase-js');
      this.client = createClient(supabaseUrl, supabaseKey);
    } catch (error) {
      throw new Error(
        'Supabase adapter requires "@supabase/supabase-js" package: npm install @supabase/supabase-js'
      );
    }
  }

  async saveGame(game: GameRecord): Promise<string> {
    const id = game.id || this.generateId();
    const now = new Date().toISOString();

    const { data, error } = await this.client
      .from('games')
      .upsert(
        {
          id,
          hpgn: game.hpgn,
          hfen: game.hfen,
          players: game.players ?? null,
          metadata: game.metadata ?? null,
          result: game.result ?? null,
          user_id: game.userId ?? null,
          synced: game.synced ?? true,
          created_at: game.createdAt || now,
          updated_at: now,
        },
        { onConflict: 'id' }
      )
      .select()
      .single();

    if (error) {
      throw new Error(`Supabase saveGame failed: ${error.message}`);
    }

    return data.id;
  }

  async loadGame(id: string): Promise<GameRecord> {
    const { data, error } = await this.client.from('games').select('*').eq('id', id).maybeSingle();

    if (error) {
      throw new Error(`Supabase loadGame failed: ${error.message}`);
    }
    if (!data) {
      throw new GameNotFoundError(id);
    }

    return this.rowToRecord(data);
  }

  async deleteGame(id: string): Promise<void> {
    const { data, error } = await this.client.from('games').delete().eq('id', id).select();

    if (error) {
      throw new Error(`Supabase deleteGame failed: ${error.message}`);
    }
    if (!data || data.length === 0) {
      throw new GameNotFoundError(id);
    }
  }

  async listGames(options?: GameQueryOptions): Promise<GameRecord[]> {
    let q = this.client.from('games').select('*');

    if (options?.userId) q = q.eq('user_id', options.userId);
    if (options?.result) q = q.eq('result', options.result);

    q = q.order('created_at', { ascending: options?.sort === 'asc' });

    if (options?.limit !== undefined) {
      const from = options?.offset ?? 0;
      q = q.range(from, from + options.limit - 1);
    } else if (options?.offset) {
      q = q.range(options.offset, options.offset + 999);
    }

    const { data, error } = await q;

    if (error) {
      throw new Error(`Supabase listGames failed: ${error.message}`);
    }

    return (data ?? []).map((row: any) => this.rowToRecord(row));
  }

  async countGames(options?: GameQueryOptions): Promise<number> {
    let q = this.client.from('games').select('*', { count: 'exact', head: true });

    if (options?.userId) q = q.eq('user_id', options.userId);
    if (options?.result) q = q.eq('result', options.result);

    const { count, error } = await q;

    if (error) {
      throw new Error(`Supabase countGames failed: ${error.message}`);
    }

    return count ?? 0;
  }

  watch(id: string, callback: (game: GameRecord) => void): Unsubscribe {
    const channelName = `games:${id}`;
    const channel = this.client
      .channel(channelName)
      .on(
        'postgres_changes',
        { event: 'UPDATE', schema: 'public', table: 'games', filter: `id=eq.${id}` },
        (payload: any) => callback(this.rowToRecord(payload.new))
      )
      .subscribe();

    this.channels.set(channelName, channel);

    return () => {
      this.client.removeChannel(channel);
      this.channels.delete(channelName);
    };
  }

  async isHealthy(): Promise<boolean> {
    try {
      const { error } = await this.client.from('games').select('id', { head: true, count: 'exact' }).limit(1);
      return !error;
    } catch {
      return false;
    }
  }

  async exportAll(): Promise<GameRecord[]> {
    const { data, error } = await this.client.from('games').select('*').order('created_at', { ascending: false });

    if (error) {
      throw new Error(`Supabase exportAll failed: ${error.message}`);
    }

    return (data ?? []).map((row: any) => this.rowToRecord(row));
  }

  async importGames(games: GameRecord[]): Promise<number> {
    let count = 0;

    for (const game of games) {
      try {
        await this.saveGame(game);
        count++;
      } catch (error) {
        console.warn(`Failed to import game ${game.id}:`, error);
      }
    }

    return count;
  }

  async clear(): Promise<number> {
    const { data, error } = await this.client.from('games').delete().neq('id', '').select();

    if (error) {
      throw new Error(`Supabase clear failed: ${error.message}`);
    }

    this.channels.forEach((channel) => this.client.removeChannel(channel));
    this.channels.clear();

    return data?.length ?? 0;
  }

  private rowToRecord(row: any): GameRecord {
    return {
      id: row.id,
      hpgn: row.hpgn,
      hfen: row.hfen,
      players: row.players ?? undefined,
      metadata: row.metadata ?? undefined,
      result: row.result ?? undefined,
      userId: row.user_id ?? undefined,
      synced: row.synced ?? undefined,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }

  private generateId(): string {
    return `game_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  // Row Level Security - games scoped to user
  async getAuthenticatedUser(): Promise<string | null> {
    const { data, error } = await this.client.auth.getUser();

    if (error || !data?.user) {
      return null;
    }

    return data.user.id;
  }
}
