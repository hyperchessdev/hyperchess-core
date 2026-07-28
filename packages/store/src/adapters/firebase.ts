import { GameStore } from '../types/game-store';
import { GameRecord, GameQueryOptions, GameNotFoundError, Unsubscribe } from '../types/game-record';

/**
 * Firebase Firestore adapter for real-time multiplayer
 * Requires: npm install firebase
 *
 * Features:
 * - Real-time updates via onSnapshot
 * - Server-side count (getCountFromServer) with client fallback
 * - Batched deletes (respects Firestore's 500-op batch limit)
 */
export class FirebaseGameStore implements GameStore {
  private db: any; // firebase.firestore.Firestore
  private fs: any; // firebase/firestore module (getFirestore, doc, collection, ...)

  constructor(firebaseApp: any) {
    try {
      const firestoreApi = require('firebase/firestore');
      this.fs = firestoreApi;
      this.db = firestoreApi.getFirestore(firebaseApp);
    } catch (error) {
      throw new Error('Firebase adapter requires "firebase" package: npm install firebase');
    }
  }

  private collectionRef() {
    return this.fs.collection(this.db, 'games');
  }

  async saveGame(game: GameRecord): Promise<string> {
    const { doc, getDoc, setDoc } = this.fs;
    const id = game.id || this.generateId();
    const now = new Date().toISOString();
    const ref = doc(this.db, 'games', id);

    const existing = await getDoc(ref);
    const createdAt = existing.exists() ? existing.data().createdAt : game.createdAt || now;

    await setDoc(
      ref,
      {
        hpgn: game.hpgn,
        hfen: game.hfen,
        players: game.players ?? null,
        metadata: game.metadata ?? null,
        result: game.result ?? null,
        userId: game.userId ?? null,
        synced: game.synced ?? true,
        createdAt,
        updatedAt: now,
      },
      { merge: true }
    );

    return id;
  }

  async loadGame(id: string): Promise<GameRecord> {
    const { doc, getDoc } = this.fs;
    const snap = await getDoc(doc(this.db, 'games', id));

    if (!snap.exists()) {
      throw new GameNotFoundError(id);
    }

    return this.docToRecord(id, snap.data());
  }

  async deleteGame(id: string): Promise<void> {
    const { doc, getDoc, deleteDoc } = this.fs;
    const ref = doc(this.db, 'games', id);
    const snap = await getDoc(ref);

    if (!snap.exists()) {
      throw new GameNotFoundError(id);
    }

    await deleteDoc(ref);
  }

  async listGames(options?: GameQueryOptions): Promise<GameRecord[]> {
    const { query, where, orderBy, limit: fsLimit, getDocs } = this.fs;
    const constraints: any[] = [];

    if (options?.userId) constraints.push(where('userId', '==', options.userId));
    if (options?.result) constraints.push(where('result', '==', options.result));
    constraints.push(orderBy('createdAt', options?.sort === 'asc' ? 'asc' : 'desc'));

    // Firestore has no offset primitive; over-fetch to (limit + offset) and slice client-side.
    const fetchLimit = (options?.limit ?? 0) + (options?.offset ?? 0);
    if (fetchLimit > 0) constraints.push(fsLimit(fetchLimit));

    const snap = await getDocs(query(this.collectionRef(), ...constraints));
    let rows = snap.docs.map((d: any) => this.docToRecord(d.id, d.data()));

    if (options?.offset) rows = rows.slice(options.offset);
    if (options?.limit) rows = rows.slice(0, options.limit);

    return rows;
  }

  async countGames(options?: GameQueryOptions): Promise<number> {
    const { query, where, getCountFromServer, getDocs } = this.fs;
    const constraints: any[] = [];

    if (options?.userId) constraints.push(where('userId', '==', options.userId));
    if (options?.result) constraints.push(where('result', '==', options.result));

    const q = query(this.collectionRef(), ...constraints);

    if (getCountFromServer) {
      const snap = await getCountFromServer(q);
      return snap.data().count;
    }

    const snap = await getDocs(q);
    return snap.size;
  }

  watch(id: string, callback: (game: GameRecord) => void): Unsubscribe {
    const { doc, onSnapshot } = this.fs;
    return onSnapshot(doc(this.db, 'games', id), (snap: any) => {
      if (snap.exists()) {
        callback(this.docToRecord(id, snap.data()));
      }
    });
  }

  async isHealthy(): Promise<boolean> {
    try {
      const { query, limit: fsLimit, getDocs } = this.fs;
      await getDocs(query(this.collectionRef(), fsLimit(1)));
      return true;
    } catch {
      return false;
    }
  }

  async exportAll(): Promise<GameRecord[]> {
    const { query, orderBy, getDocs } = this.fs;
    const snap = await getDocs(query(this.collectionRef(), orderBy('createdAt', 'desc')));
    return snap.docs.map((d: any) => this.docToRecord(d.id, d.data()));
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
    const { doc, getDocs, writeBatch } = this.fs;
    const snap = await getDocs(this.collectionRef());
    const docs = snap.docs;

    const BATCH_LIMIT = 500; // Firestore's max writes per batch
    let deleted = 0;

    for (let i = 0; i < docs.length; i += BATCH_LIMIT) {
      const batch = writeBatch(this.db);
      const chunk = docs.slice(i, i + BATCH_LIMIT);
      for (const d of chunk) {
        batch.delete(doc(this.db, 'games', d.id));
      }
      await batch.commit();
      deleted += chunk.length;
    }

    return deleted;
  }

  private docToRecord(id: string, data: any): GameRecord {
    return {
      id,
      hpgn: data.hpgn,
      hfen: data.hfen,
      players: data.players ?? undefined,
      metadata: data.metadata ?? undefined,
      result: data.result ?? undefined,
      userId: data.userId ?? undefined,
      synced: data.synced ?? undefined,
      createdAt: data.createdAt,
      updatedAt: data.updatedAt,
    };
  }

  private generateId(): string {
    return `game_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
}
