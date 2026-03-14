"use client";

import { useCallback, useState } from "react";

interface AudioPlaybackState {
  readonly playing: boolean;
  readonly togglePlayback: () => void;
}

export function useAudioPlayback(): AudioPlaybackState {
  const [playing, setPlaying] = useState(false);

  const togglePlayback = useCallback(() => {
    setPlaying((prev) => {
      const next = !prev;
      console.log(
        next
          ? "Audio playback not implemented yet"
          : "Audio stop not implemented yet"
      );
      return next;
    });
  }, []);

  return { playing, togglePlayback };
}
