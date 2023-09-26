import de.ovgu.featureide.fm.core.base.IFeatureModel;
import de.ovgu.featureide.fm.core.base.impl.FMFormatManager;
import de.ovgu.featureide.fm.core.init.FMCoreLibrary;
import de.ovgu.featureide.fm.core.init.LibraryManager;
import de.ovgu.featureide.fm.core.io.IFeatureModelFormat;
import de.ovgu.featureide.fm.core.io.dimacs.DIMACSFormat;
import de.ovgu.featureide.fm.core.io.manager.FeatureModelIO;
import de.ovgu.featureide.fm.core.io.manager.FeatureModelManager;
import de.ovgu.featureide.fm.core.io.uvl.UVLFeatureModelFormat;
import de.ovgu.featureide.fm.core.io.xml.XmlFeatureModelFormat;

import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Scanner;

public class Main {
    public static void main(String[] args) {
        if (args.length > 2)
            throw new RuntimeException("usage: java -jar io.jar [file|-] [uvl|xml|model|cnf|dimacs|sat]");

        LibraryManager.registerLibrary(FMCoreLibrary.getInstance());
        FMFormatManager.getInstance().addExtension(new ModelFormat());
        FMFormatManager.getInstance().addExtension(new SatFormat());

        IFeatureModel featureModel;
        if (args.length > 0 && !args[0].startsWith("-")) {
            Path inputPath = Paths.get(args[0]);
            featureModel = FeatureModelManager.load(inputPath);
        } else {
            StringBuilder sb = new StringBuilder();
            Scanner sc = new Scanner(System.in);
            while (sc.hasNextLine()) {
                sb.append(sc.nextLine());
                sb.append('\n');
            }
            featureModel = FeatureModelIO.getInstance()
                    .loadFromSource(sb, Paths.get(args.length > 0 ? args[0].replace("cnf", "dimacs") : "-.uvl"));
        }
        if (featureModel == null)
            throw new RuntimeException("failed to load feature model");

        IFeatureModelFormat format = new ModelFormat();
        if (args.length == 2) {
            String formatString = args[1];
            switch (formatString) {
                case "uvl":
                    format = new UVLFeatureModelFormat();
                    break;
                case "xml":
                    format = new XmlFeatureModelFormat();
                    break;
                case "model":
                    format = new ModelFormat();
                    break;
                case "cnf":
                case "dimacs":
                    format = new DIMACSFormat();
                    break;
                case "sat":
                    format = new SatFormat();
                    break;
                default:
                    throw new RuntimeException("invalid format");
            }
        }

        String output = format.getInstance().write(featureModel);
        System.out.print(output);
    }
}
