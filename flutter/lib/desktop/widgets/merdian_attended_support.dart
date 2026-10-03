import 'package:flutter/material.dart';

const merdianAppName = 'Merdian-Desk';
const merdianTeal = Color(0xFF006B64);
const merdianLightTeal = Color(0xFF8DE3D8);

ThemeData merdianTheme(ThemeData base) {
  final accent =
      base.brightness == Brightness.dark ? merdianLightTeal : merdianTeal;
  return base.copyWith(
    colorScheme: base.colorScheme.copyWith(
      primary: accent,
      secondary: accent,
      onPrimary: base.brightness == Brightness.dark
          ? const Color(0xFF003C37)
          : Colors.white,
    ),
    textButtonTheme: TextButtonThemeData(
      style: base.textButtonTheme.style?.copyWith(
        foregroundColor: WidgetStatePropertyAll(accent),
      ),
    ),
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: base.elevatedButtonTheme.style?.copyWith(
        backgroundColor: const WidgetStatePropertyAll(merdianTeal),
        foregroundColor: const WidgetStatePropertyAll(Colors.white),
      ),
    ),
  );
}

class MerdianMark extends StatelessWidget {
  final double size;
  const MerdianMark({super.key, this.size = 32});

  @override
  Widget build(BuildContext context) => Semantics(
        label: merdianAppName,
        image: true,
        child: SizedBox.square(
          dimension: size,
          child: CustomPaint(painter: _MerdianMarkPainter()),
        ),
      );
}

class _MerdianMarkPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRRect(
      RRect.fromRectAndRadius(
          Offset.zero & size, Radius.circular(size.width / 4)),
      Paint()..color = merdianTeal,
    );
    final path = Path()
      ..moveTo(size.width * .24, size.height * .72)
      ..lineTo(size.width * .24, size.height * .28)
      ..lineTo(size.width * .5, size.height * .55)
      ..lineTo(size.width * .76, size.height * .28)
      ..lineTo(size.width * .76, size.height * .72);
    canvas.drawPath(
      path,
      Paint()
        ..color = Colors.white
        ..style = PaintingStyle.stroke
        ..strokeWidth = size.width * .09
        ..strokeCap = StrokeCap.round
        ..strokeJoin = StrokeJoin.round,
    );
  }

  @override
  bool shouldRepaint(_MerdianMarkPainter oldDelegate) => false;
}

class MerdianBrandBadge extends StatelessWidget {
  const MerdianBrandBadge({super.key});

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.fromLTRB(16, 18, 16, 6),
        child: Row(
          children: [
            const ExcludeSemantics(child: MerdianMark()),
            const SizedBox(width: 10),
            Expanded(
              child: Semantics(
                header: true,
                child: Text(
                  merdianAppName,
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                ),
              ),
            ),
          ],
        ),
      );
}

class MerdianSupportGuide extends StatelessWidget {
  final VoidCallback onExit;
  const MerdianSupportGuide({super.key, required this.onExit});

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.fromLTRB(16, 16, 16, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Semantics(
              header: true,
              child: Text('Support in three steps',
                  style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w700,
                      )),
            ),
            const SizedBox(height: 12),
            const _SupportStep('1', 'Share your ID',
                'Read the ID above to the colleague you expect.'),
            const _SupportStep('2', 'Request support',
                'Your colleague enters your ID and chooses Connect.'),
            const _SupportStep('3', 'You choose Accept',
                'Check who is connecting. Nothing starts until you Accept.'),
            const Divider(height: 20),
            const Text(
              'End support with Disconnect in your session window. '
              'Exit the app when you are finished.',
              style: TextStyle(fontSize: 12, height: 1.4),
            ),
            const SizedBox(height: 10),
            OutlinedButton.icon(
              onPressed: onExit,
              icon: const Icon(Icons.close, size: 16),
              label: const Text('Exit Merdian-Desk'),
            ),
            const SizedBox(height: 6),
            const Text('Windows 11 · Attended support',
                style: TextStyle(fontSize: 11)),
          ],
        ),
      );
}

class _SupportStep extends StatelessWidget {
  final String number;
  final String title;
  final String description;
  const _SupportStep(this.number, this.title, this.description);

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.only(bottom: 12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              width: 22,
              height: 22,
              alignment: Alignment.center,
              decoration: const BoxDecoration(
                  color: merdianTeal, shape: BoxShape.circle),
              child: Text(number,
                  style: const TextStyle(color: Colors.white, fontSize: 12)),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title,
                      style: const TextStyle(
                          fontWeight: FontWeight.w600, fontSize: 13)),
                  const SizedBox(height: 3),
                  Text(description,
                      style: const TextStyle(fontSize: 12, height: 1.4)),
                ],
              ),
            ),
          ],
        ),
      );
}
