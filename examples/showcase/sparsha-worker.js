const canceledTasks = new Set();

self.onmessage = async (event) => {
  const message = event.data;
  if (!message || typeof message !== "object") {
    return;
  }

  if (message.type === "cancel") {
    canceledTasks.add(message.task_id);
    return;
  }

  if (message.type !== "run") {
    return;
  }

  const { task_id, task_kind, task_key, generation, payload_json } = message;

  try {
    const payload = parsePayload(payload_json);
    const resultPayload = await executeTask(task_kind, payload);

    if (canceledTasks.has(task_id)) {
      canceledTasks.delete(task_id);
      self.postMessage({
        type: "canceled",
        task_id,
        task_kind,
        task_key,
        generation,
      });
      return;
    }

    self.postMessage({
      type: "done",
      task_id,
      task_kind,
      task_key,
      generation,
      payload_json: JSON.stringify(resultPayload),
    });
  } catch (error) {
    canceledTasks.delete(task_id);
    self.postMessage({
      type: "error",
      task_id,
      task_kind,
      task_key,
      generation,
      message: String(error),
    });
  }
};

function parsePayload(payloadJson) {
  if (typeof payloadJson !== "string" || payloadJson.length === 0) {
    return {};
  }

  try {
    return JSON.parse(payloadJson);
  } catch (_) {
    return {};
  }
}

async function executeTask(taskKind, payload) {
  switch (taskKind) {
    case "echo":
      return payload;

    case "sleep_echo": {
      const millis = Number(payload?.millis ?? 0);
      await sleep(Math.max(0, millis));
      return payload?.data ?? payload;
    }

    case "analyze_text": {
      const text = String(payload?.text ?? "");
      return analyzeText(text);
    }

    default:
      throw new Error(`unknown task kind: ${taskKind}`);
  }
}

function analyzeText(text) {
  const trimmed = text.trim();
  const words = trimmed.length === 0 ? 0 : trimmed.split(/\s+/).length;
  const lines = text.length === 0 ? 0 : text.split(/\r?\n/).length;
  const chars = Array.from(text).length;
  const preview = Array.from(text).slice(0, 48).join("");

  return {
    word_count: words,
    line_count: lines,
    char_count: chars,
    preview,
  };
}

function sleep(ms) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}
