import 'dart:io';

/// End this attended host process rather than hiding its main window.
Future<void> closeMerdianDesk({
  required Future<void> Function() closeOwnedSessions,
  void Function(int)? terminate,
}) async {
  try {
    await closeOwnedSessions();
  } finally {
    (terminate ?? exit)(0);
  }
}
