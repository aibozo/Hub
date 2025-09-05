"use client";
import { create } from 'zustand';

export type ActivityItem = { ts: number; kind: 'tool_call' | 'tool_result' | 'error' | 'info'; payload: any };

interface UiState {
  rightOpen: boolean;
  toggleRight: () => void;
  activity: ActivityItem[];
  pushActivity: (item: ActivityItem) => void;
  clearActivity: () => void;
  selectedChatId: string | null;
  setSelectedChatId: (id: string | null) => void;
  selectedAgentId: string | null;
  setSelectedAgentId: (id: string | null) => void;
}

export const useUi = create<UiState>((set) => ({
  rightOpen: false,
  toggleRight: () => set((s) => ({ rightOpen: !s.rightOpen })),
  activity: [],
  pushActivity: (item) => set((s) => ({ activity: [{ ...item }, ...s.activity].slice(0, 200) })),
  clearActivity: () => set({ activity: [] }),
  selectedChatId: null,
  setSelectedChatId: (id) => set({ selectedChatId: id }),
  selectedAgentId: null,
  setSelectedAgentId: (id) => set({ selectedAgentId: id }),
}));
